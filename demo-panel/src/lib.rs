//! The page-hosted half of the demo: a button and a number.
//!
//! The mount activation builds the view and wires the button's press to a sync
//! port; a press publishes one click; a delivery on `total` sets the number.
//! It holds `dom` for exactly that, and reaches every capability through the
//! generated module, so deleting a word from the specification breaks this
//! compile.

mod spec;

use std::cell::RefCell;

use brenn_guest::{Activation, Error, OutPort, Processor};
use serde::{Deserialize, Serialize};

use crate::spec::dom;

/// The sync port the button's press arrives on. Not an input port name; must
/// collide with none.
const PRESS_PORT: dom::SyncPort = dom::SyncPort("press");

/// A component-authored marker for the button, never a styling hook.
const PRESS_MARKER: &str = "data-demo-press";

/// A component-authored marker for the total display, never a styling hook.
const TOTAL_MARKER: &str = "data-demo-total";

/// What a press publishes. The port is doctyped `brenn.demo.click@1`, whose body
/// carries nothing today: the click is the whole fact.
#[derive(Serialize)]
struct Click {}

impl spec::ClicksPayload for Click {}

/// What arrives on `total`, published by whichever counter this deployment
/// wired — the one on this page, or the one on the backend.
#[derive(Deserialize)]
struct Total {
    total: u64,
}

const CLICKS: OutPort<Click> = spec::clicks();

// One instantiation backs one instance for the page's lifetime, so the view
// handles are ordinary interior-mutable module state. A handle names the same
// element on every activation after the one that created it.
thread_local! {
    static PANEL: RefCell<Option<View>> = const { RefCell::new(None) };
}

/// The elements an activation writes to, built by the mount activation.
struct View {
    total: dom::Node,
}

struct DemoPanel;

impl Processor for DemoPanel {
    fn receive(activation: Activation) -> Result<Option<String>, Error> {
        PANEL.with(|panel| on_activation(&activation, &mut panel.borrow_mut()))?;
        // The mount call has no reply dialect and the press is not a default
        // action this component cancels, so both are answered with nothing.
        Ok(None)
    }
}

/// Handle one activation: build the view when this is the mount call, publish
/// when a press asked for it, then render whatever total was delivered.
///
/// A mount activation windows whatever input was already pending, so the build
/// and the render both run on it — a component is never told why it woke.
fn on_activation(activation: &Activation, panel: &mut Option<View>) -> Result<(), Error> {
    if activation.sync_is(dom::MOUNT) {
        *panel = Some(build_view());
    } else if let Some(port) = activation.sync() {
        on_gesture(port)?;
    }
    let view = panel
        .as_ref()
        .expect("the mount activation builds the view before any other call");
    for window in activation.delivered_windows() {
        let spec::InPort::Total = spec::InPort::of(window)?;
        if let Some(envelope) = window.new_envelopes().last() {
            let envelope = envelope?;
            let total: Total = serde_json::from_str(&envelope.body).map_err(|e| {
                Error::failed(format!(
                    "the delivered total {:?} is not a `brenn.demo.total@1` body: {e}",
                    envelope.body
                ))
            })?;
            dom::set_text(view.total, &total.total.to_string());
        }
    }
    Ok(())
}

/// Answer a press: one click on the bus.
///
/// A refusal other than quota is structural — an unbound port, a body over the
/// cap — and no later press repairs it, so the first one takes the instance's
/// error card rather than being logged and forgotten.
fn on_gesture(port: &str) -> Result<(), Error> {
    if port != PRESS_PORT {
        return Err(Error::failed(format!(
            "the panel wired no gesture to sync port {port:?}"
        )));
    }
    match CLICKS.publish(&Click {}) {
        Ok(()) => Ok(()),
        Err(err) if err.is_quota() => {
            spec::log::warn("a press was dropped: the publish quota is exhausted");
            Ok(())
        }
        Err(err) => Err(err),
    }
}

/// Build the view under this instance's host element and wire the press.
///
/// Called once, from the mount activation. The listener is the kernel's and is
/// page-lifetime: each press arrives as a sync-call activation on its port.
fn build_view() -> View {
    let root = dom::root();

    let button = dom::marked("button", PRESS_MARKER);
    dom::set_text(button, "press");
    let total = dom::marked("span", TOTAL_MARKER);
    // The number the page shows before any counter has published one. A blank
    // element would be indistinguishable from a counter that never ran.
    dom::set_text(total, "0");

    dom::append(root, button);
    dom::append(root, total);
    dom::listen(button, "click", PRESS_PORT);

    View { total }
}

brenn_guest::export_processor!(DemoPanel);
