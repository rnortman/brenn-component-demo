//! The placement-agnostic half of the demo: a running click total.
//!
//! It keeps no state of its own. The total it last published is the retained
//! message on its `total` port, read back at the top of the next activation and
//! added to the count of new clicks. Both hosts instantiate per activation, so
//! linear memory would not have carried it anyway; a component whose whole
//! memory is a retained channel cannot tell which host it got, which is what
//! lets this one artifact ship for both placements.

mod spec;

use brenn_guest::{Activation, Error, OutPort, Processor};
use serde::{Deserialize, Serialize};

/// What travels on `total`, in both directions: this component publishes it and
/// reads its own last one back.
///
/// An object rather than a bare number so the doctype has somewhere to grow.
#[derive(Serialize, Deserialize)]
struct Total {
    total: u64,
}

impl spec::TotalPayload for Total {}

const TOTAL: OutPort<Total> = spec::total();

struct DemoCounter;

impl Processor for DemoCounter {
    fn receive(activation: Activation) -> Result<Option<String>, Error> {
        let mut running = 0u64;
        let mut clicks = 0u64;
        for window in activation.port_windows() {
            // Matched through the specification enum, so a port rename fails at
            // build time rather than at runtime on whichever host is running it.
            match spec::InPort::of(window)? {
                spec::InPort::Clicks => clicks += window.new_raw().len() as u64,
                spec::InPort::Total => running = latest_total(window)?,
            }
        }
        let total = Total {
            total: running + clicks,
        };
        spec::log::info(format!(
            "{clicks} new click(s) on a running total of {running}"
        ));
        TOTAL.publish(&total)?;
        Ok(None)
    }
}

/// The last total in the window, whether the host offered it as context or as
/// new.
///
/// Which of the two it is depends on the binding: at `push_depth = 0` the
/// component never wakes on its own publish, so the retained message is context
/// by the time anything else does wake it — but a first activation that arrives
/// alongside it would see it as new, and the arithmetic is the same either way.
/// Zero where the window is empty: nothing has been published yet.
fn latest_total(window: &brenn_guest::PortWindow) -> Result<u64, Error> {
    let Some(envelope) = window
        .context_envelopes()
        .chain(window.new_envelopes())
        .last()
    else {
        return Ok(0);
    };
    let envelope = envelope?;
    let total: Total = serde_json::from_str(&envelope.body).map_err(|e| {
        // The port is doctyped and the only publisher is this component, so a
        // body that does not parse is a deployment that bound the port to
        // something else — structural, and the activation fails.
        Error::failed(format!(
            "the retained total {:?} is not a `brenn.demo.total@1` body: {e}",
            envelope.body
        ))
    })?;
    Ok(total.total)
}

brenn_guest::export_processor!(DemoCounter);
