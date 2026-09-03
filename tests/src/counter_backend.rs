//! The counter on the backend host: wasmtime, through the same
//! `ProcessorComponent` a running brenn loads a packaged component with.
//!
//! The engine, linker, grant enforcement, publish budget, and envelope parser
//! are brenn's own, and so are the envelope frames the windows carry; only the
//! load spec is test-supplied.

use std::collections::HashMap;

use brenn_envelope::testutils::{NOW_MS, envelope};
use brenn_wasm::{
    ComponentGrant, ProcessorActivation, ProcessorComponent, ProcessorLoadSpec, ProcessorOutcome,
    ProcessorPortWindow, ProcessorReceiveError, store::DEFAULT_MAX_PAGE_COUNT,
};
use brenn_wasm_dispatch::tests::{allow_all, noop_proc_alerter, test_out_spec};

use crate::{
    CLICKS_PORT, EXPECTED, SCRIPT, Step, TOTAL_PORT, TotalWindow, WRONG_DOCTYPE_BODY, artifact,
    total_body,
};

/// Where a deployment's `io total` would land. Any address does: what the test
/// asserts is the body and the port that carried it, and the port is recovered
/// by inverting the output map below.
const TOTAL_CHANNEL: &str = "ephemeral:demo.total";

/// The click envelope: the doctype's body carries nothing, so the click is the
/// whole fact.
const CLICK_BODY: &str = "{}";

fn load() -> ProcessorComponent {
    let output_ports = HashMap::from([(
        TOTAL_PORT.to_string(),
        test_out_spec(TOTAL_CHANNEL.to_string()),
    )]);
    ProcessorComponent::load(ProcessorLoadSpec {
        component_path: &artifact("DEMO_COUNTER_WASM"),
        slug: "demo-counter",
        declared_out_ports: output_ports.keys().cloned().collect(),
        output_ports,
        // Must list every port an activation window can name; a missing entry
        // panics at activation time.
        input_amplification_mt: HashMap::from([
            (CLICKS_PORT.to_string(), 1000u64),
            (TOTAL_PORT.to_string(), 1000u64),
        ]),
        mqtt_sinks: HashMap::new(),
        config: HashMap::new(),
        // Must match the specification's `requires`; a missing grant fails the
        // load.
        grants: [ComponentGrant::Ports, ComponentGrant::Log]
            .into_iter()
            .collect(),
        store_path: None,
        max_page_count: DEFAULT_MAX_PAGE_COUNT,
        max_payload_bytes: 1024 * 1024,
        alerter: noop_proc_alerter(),
        output_acl: allow_all(),
        mqtt_publish: None,
        tool_host: None,
    })
}

/// The `total` window this step offers, in this host's window type.
fn total_window(total: &TotalWindow) -> Option<ProcessorPortWindow> {
    let (envelopes, new_from) = match total {
        TotalWindow::Absent => return None,
        TotalWindow::Empty => (vec![], 0),
        TotalWindow::Context(total) => (vec![envelope("retained", &total_body(*total))], 1),
        TotalWindow::New(total) => (vec![envelope("retained", &total_body(*total))], 0),
    };
    Some(ProcessorPortWindow {
        port: TOTAL_PORT.to_string(),
        envelopes,
        new_from,
        dropped: 0,
    })
}

fn activation(step: &Step) -> ProcessorActivation {
    let mut ports = vec![ProcessorPortWindow {
        port: CLICKS_PORT.to_string(),
        envelopes: (0..step.new_clicks)
            .map(|i| envelope(&format!("click{i}"), CLICK_BODY))
            .collect(),
        new_from: 0,
        dropped: 0,
    }];
    ports.extend(total_window(&step.total));
    ProcessorActivation {
        ports,
        deferred: vec![],
        now: Some(NOW_MS),
        sync: None,
    }
}

/// Reduces this host's outcome to `(port, body)` pairs.
fn published(outcome: ProcessorOutcome) -> Vec<(String, String)> {
    match outcome {
        ProcessorOutcome::Ok { publishes, .. } => publishes
            .into_iter()
            .map(|publish| {
                assert_eq!(
                    publish.channel_address, TOTAL_CHANNEL,
                    "the counter publishes on its one output port"
                );
                assert_eq!(
                    publish.deliver_after, None,
                    "the counter schedules nothing for later"
                );
                (TOTAL_PORT.to_string(), publish.payload)
            })
            .collect(),
        other => panic!("the activation must succeed, got {other:?}"),
    }
}

#[test]
fn the_script_publishes_the_expected_totals_on_the_backend() {
    let component = load();
    let actual: Vec<(String, String)> = SCRIPT
        .iter()
        .flat_map(|step| published(component.handle(activation(step))))
        .collect();

    let expected: Vec<(String, String)> = EXPECTED
        .iter()
        .map(|(port, body)| (port.to_string(), body.to_string()))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn a_click_with_no_retained_total_publishes_the_click_count() {
    // The first step alone, driven on its own instance: a component whose whole
    // memory is a retained channel must still answer the activation that has no
    // retained message yet, and answering it with nothing would leave the page
    // showing a number no publisher ever sent.
    let component = load();
    let actual = published(component.handle(activation(&SCRIPT[0])));
    assert_eq!(
        actual,
        vec![(TOTAL_PORT.to_string(), EXPECTED[0].1.to_string())]
    );
}

#[test]
fn a_total_that_is_not_the_doctype_is_refused() {
    // Reading an unparseable total as zero would make a deployment that bound
    // `total` to another channel look like a counter that keeps restarting.
    let component = load();
    let outcome = component.handle(ProcessorActivation {
        ports: vec![ProcessorPortWindow {
            port: TOTAL_PORT.to_string(),
            envelopes: vec![envelope("wrong", WRONG_DOCTYPE_BODY)],
            new_from: 0,
            dropped: 0,
        }],
        deferred: vec![],
        now: Some(NOW_MS),
        sync: None,
    });

    match outcome {
        ProcessorOutcome::Err(ProcessorReceiveError::ProcessingFailed(why)) => assert!(
            why.contains("brenn.demo.total@1"),
            "the refusal names the doctype the body is not: {why}",
        ),
        other => panic!("the guest refuses this itself; the host does not: {other:?}"),
    }
}
