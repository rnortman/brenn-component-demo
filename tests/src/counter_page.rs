//! The counter on the page host: the same artifact, the same script, the same
//! published bodies.
//!
//! Both hosts instantiate per activation, so neither would carry a total in
//! linear memory: the retained `total` port is the counter's memory on both.
//! What the two hosts differ in is how they window that retained message —
//! context on the backend's sampled port, context or news on the page — which
//! is what [`crate::TotalWindow`] models and why both suites exist.

use brenn_envelope::grants::ComponentGrant;
use brenn_envelope::testutils::{NOW_MS, envelope};
use brenn_page_harness::{Harness, Page, types};

use crate::{
    CLICKS_PORT, COLD_CLICK, EXPECTED, SCRIPT, Step, TOTAL_PORT, TotalWindow, WRONG_DOCTYPE_BODY,
    artifact, total_body,
};

const CLICK_BODY: &str = "{}";

/// The `total` window this step offers, in this host's window type.
fn total_window(total: &TotalWindow) -> Option<types::PortWindow> {
    let (envelopes, new_from) = match total {
        TotalWindow::Absent => return None,
        TotalWindow::Empty => (vec![], 0),
        TotalWindow::Context(total) => (vec![envelope("retained", &total_body(*total))], 1),
        TotalWindow::New(total) => (vec![envelope("retained", &total_body(*total))], 0),
    };
    Some(types::PortWindow {
        port: TOTAL_PORT.to_string(),
        envelopes,
        new_from,
        dropped: 0,
    })
}

fn activation(step: &Step) -> types::Activation {
    let mut ports = vec![types::PortWindow {
        port: CLICKS_PORT.to_string(),
        envelopes: (0..step.new_clicks)
            .map(|i| envelope(&format!("click{i}"), CLICK_BODY))
            .collect(),
        new_from: 0,
        dropped: 0,
    }];
    ports.extend(total_window(&step.total));
    types::Activation {
        ports,
        deferred: vec![],
        now: Some(NOW_MS),
        sync: None,
    }
}

/// A counter instance linked with exactly the grants its specification requires.
///
/// A grant this list named that the specification does not would be a page more
/// permissive than the deployment it is testing.
fn loaded() -> Harness {
    Harness::new(
        &artifact("DEMO_COUNTER_WASM"),
        Page::new(),
        &[ComponentGrant::Ports, ComponentGrant::Log],
    )
}

#[test]
fn the_script_publishes_the_expected_totals_on_the_page() {
    let mut harness = loaded();
    for step in &SCRIPT {
        harness.call(activation(step));
    }

    let actual: Vec<(String, String)> = harness
        .page()
        .published
        .iter()
        .map(|(port, body)| (port.clone(), body.clone()))
        .collect();
    let expected: Vec<(String, String)> = EXPECTED
        .iter()
        .map(|(port, body)| (port.to_string(), body.to_string()))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn the_counter_reaches_the_page_host_through_ports_and_log_alone() {
    // A grant not in `requires` fails the load; an import actually called but
    // not granted would show up in the transcript here.
    let mut harness = loaded();
    harness.call(activation(&SCRIPT[COLD_CLICK]));

    let transcript = harness.transcript();
    assert!(
        transcript
            .iter()
            .any(|call| call.starts_with(&format!("ports.publish({TOTAL_PORT},"))),
        "{transcript:?}",
    );
    assert!(
        transcript.iter().any(|call| call.starts_with("log.")),
        "the counter narrates what it counted: {transcript:?}",
    );
    assert!(
        transcript
            .iter()
            .all(|call| call.starts_with("ports.") || call.starts_with("log.")),
        "{transcript:?}",
    );
}

#[test]
fn a_total_that_is_not_the_doctype_is_refused() {
    // The port is doctyped and the only publisher is the counter itself, so a
    // body that does not parse is a deployment that bound `total` to another
    // channel. Reading it as zero would restart the count silently; the
    // activation fails instead, and the instance survives to be told again.
    let mut harness = loaded();
    let error = harness.call_expecting_a_refusal(types::Activation {
        ports: vec![types::PortWindow {
            port: TOTAL_PORT.to_string(),
            envelopes: vec![envelope("wrong", WRONG_DOCTYPE_BODY)],
            new_from: 0,
            dropped: 0,
        }],
        deferred: vec![],
        now: Some(NOW_MS),
        sync: None,
    });

    match error {
        types::ReceiveError::ProcessingFailed(why) => assert!(
            why.contains("brenn.demo.total@1"),
            "the refusal names the doctype the body is not: {why}",
        ),
        other => panic!("the guest refuses this itself; the host does not: {other:?}"),
    }
    assert!(
        harness.page().published.is_empty(),
        "a refused activation publishes nothing",
    );
}
