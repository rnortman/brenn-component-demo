//! The panel on the page host: the view it builds, the click it publishes, and
//! the number it renders.
//!
//! The panel is page-only — it holds `dom` — so this is its whole host-side
//! coverage. What it is not is coverage of the *page*: the recording host
//! answers `dom` calls, it does not run a browser, so "the button is clickable"
//! and "the kernel mounted the kind at all" are the browser suite's to say.
//! What is settled here is the component's own vocabulary: which elements it
//! creates, which markers it puts on them, which port it listens with, and what
//! it publishes and renders.

use brenn_envelope::grants::ComponentGrant;
use brenn_page_harness::{Harness, Page, ROOT, delivery_on, gesture, mount, ports, types};

use crate::{WRONG_DOCTYPE_BODY, artifact, total_body};

/// The port the panel listens on. Named here rather than imported because the
/// guest is a wasm32 crate: what this test can see of it is the artifact.
const PRESS_PORT: &str = "press";

/// The panel's out port, as its specification names it.
const CLICKS_PORT: &str = "clicks";

/// The panel's in port.
const TOTAL_PORT: &str = "total";

const PRESS_MARKER: &str = "data-demo-press";
const TOTAL_MARKER: &str = "data-demo-total";

fn mounted() -> Harness {
    let mut harness = Harness::new(
        &artifact("DEMO_PANEL_WASM"),
        Page::new(),
        &[
            ComponentGrant::Ports,
            ComponentGrant::Log,
            ComponentGrant::Dom,
        ],
    );
    harness.call(mount());
    harness
}

#[test]
fn the_mount_activation_builds_the_view_and_wires_the_press() {
    let mut harness = mounted();
    let transcript = harness.transcript();

    let page = harness.page();
    let button = page.marked_child(ROOT, PRESS_MARKER);
    let total = page.marked_child(ROOT, TOTAL_MARKER);
    assert_eq!(page.text_of(button), "press");
    assert_eq!(
        page.text_of(total),
        "0",
        "the number is drawn before any counter has published one",
    );
    assert_eq!(
        page.children(ROOT),
        vec![button, total],
        "both parts hang under the instance's own host element",
    );

    assert!(
        transcript
            .iter()
            .any(|call| call == &format!("dom.listen(n{button}, click, {PRESS_PORT})")),
        "{transcript:?}",
    );
    assert!(
        transcript.iter().all(|call| !call.starts_with("page-dom.")),
        "the panel is not chrome and holds no page authority: {transcript:?}",
    );
}

#[test]
fn each_press_publishes_one_click() {
    let mut harness = mounted();
    let button = harness.page().marked_child(ROOT, PRESS_MARKER);

    harness.call(gesture(PRESS_PORT, button));
    harness.call(gesture(PRESS_PORT, button));

    assert_eq!(
        harness.page().published_on(CLICKS_PORT),
        vec!["{}", "{}"],
        "the doctype's body carries nothing; the press is the whole fact",
    );
}

#[test]
fn a_delivered_total_sets_the_number() {
    let mut harness = mounted();
    let total = harness.page().marked_child(ROOT, TOTAL_MARKER);

    let seven = total_body(7);
    harness.call(delivery_on(TOTAL_PORT, &[], &[&seven], 0));
    assert_eq!(harness.page().text_of(total), "7");

    // Latest wins: a window carrying several takes the last, which is what
    // `retain_depth = 1` on the channel delivers and what a coalesced
    // activation delivers anyway.
    let (eight, nine) = (total_body(8), total_body(9));
    harness.call(delivery_on(TOTAL_PORT, &[&seven], &[&eight, &nine], 0));
    assert_eq!(harness.page().text_of(total), "9");

    assert!(
        harness.page().published_on(CLICKS_PORT).is_empty(),
        "rendering a total is not a click",
    );
}

#[test]
fn a_gesture_on_a_port_the_panel_wired_nothing_to_is_refused() {
    // An unknown port is a wiring bug: a kind serving a page whose markup
    // listens with a name this build no longer publishes for. The panel refuses
    // rather than answering with nothing, which would make the button look wired
    // and publish nothing forever.
    let mut harness = mounted();
    let button = harness.page().marked_child(ROOT, PRESS_MARKER);

    let error = harness.call_expecting_a_refusal(gesture("nope", button));
    match error {
        types::ReceiveError::ProcessingFailed(why) => assert!(
            why.contains("nope"),
            "the refusal names the port nothing was wired to: {why}",
        ),
        other => panic!("an unwired gesture is a refusal, not a trap: {other:?}"),
    }
    assert!(
        harness.page().published_on(CLICKS_PORT).is_empty(),
        "a refused gesture publishes nothing",
    );
}

#[test]
fn a_total_that_is_not_the_doctype_is_refused() {
    // `total` is doctyped, so a body that does not parse is a deployment that
    // bound the port to another channel. The panel refuses rather than
    // rendering something, and the number keeps the last value it was told.
    let mut harness = mounted();
    let total = harness.page().marked_child(ROOT, TOTAL_MARKER);

    let seven = total_body(7);
    harness.call(delivery_on(TOTAL_PORT, &[], &[&seven], 0));

    let error =
        harness.call_expecting_a_refusal(delivery_on(TOTAL_PORT, &[], &[WRONG_DOCTYPE_BODY], 0));
    match error {
        types::ReceiveError::ProcessingFailed(why) => assert!(
            why.contains("brenn.demo.total@1"),
            "the refusal names the doctype the body is not: {why}",
        ),
        other => panic!("the guest refuses this itself; the host does not: {other:?}"),
    }
    assert_eq!(
        harness.page().text_of(total),
        "7",
        "a refused delivery renders nothing, so the number is the last one delivered",
    );
}

#[test]
fn a_press_the_publish_quota_refuses_is_logged_and_survived() {
    // A quota refusal is the one publish failure a later press repairs: the
    // budget refills. So it costs this press and nothing else — no error card
    // for a user who pressed the button quickly.
    let mut harness = mounted();
    let button = harness.page().marked_child(ROOT, PRESS_MARKER);
    harness.page().publish_answer = Some(ports::PublishError::QuotaExceeded);

    harness.call(gesture(PRESS_PORT, button));

    let transcript = harness.transcript();
    assert!(
        transcript
            .iter()
            .any(|call| call.starts_with("log.warn") && call.contains("quota")),
        "the dropped press is narrated rather than silent: {transcript:?}",
    );
    assert!(
        harness.page().published_on(CLICKS_PORT).is_empty(),
        "a refused publish carries nothing onto the bus",
    );
}
