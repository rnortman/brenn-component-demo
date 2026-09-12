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

/// The panel's `io state` port.
const STATE_PORT: &str = "state";

const PRESS_MARKER: &str = "data-demo-press";
const TOTAL_MARKER: &str = "data-demo-total";

/// A panel instance whose state port the harness carries, before its mount
/// activation. The artifact is instantiated per activation, so a script that
/// did not carry the port would hand every activation after the first a panel
/// that had never mounted.
fn unmounted() -> Harness {
    Harness::new(
        &artifact("DEMO_PANEL_WASM"),
        Page::new(),
        &[
            ComponentGrant::Ports,
            ComponentGrant::Log,
            ComponentGrant::Dom,
        ],
    )
    .retaining_state(STATE_PORT)
}

fn mounted() -> Harness {
    let mut harness = unmounted();
    harness.call(mount());
    harness
}

/// The state body for a panel whose display is `display`, as the guest's
/// `Panel` serializes: `dom::Node` is transparent over the bare handle.
fn state_body(display: u64) -> String {
    format!(r#"{{"display":{display}}}"#)
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

#[test]
fn the_mount_activation_stores_the_display_handle_and_later_ones_store_nothing() {
    // The whole of the panel's memory, and the whole of what it costs: one
    // state body at the mount, and none after it, because the handle never
    // changes and the store elides a duplicate.
    let mut harness = mounted();
    let total = harness.page().marked_child(ROOT, TOTAL_MARKER);
    let button = harness.page().marked_child(ROOT, PRESS_MARKER);

    harness.call(gesture(PRESS_PORT, button));
    harness.call(delivery_on(TOTAL_PORT, &[], &[&total_body(4)], 0));

    assert_eq!(
        harness.page().published_on(STATE_PORT),
        vec![state_body(total)],
        "the mount stores the display it built, and nothing after it changes the state",
    );
}

#[test]
fn a_total_delivered_after_a_press_is_rendered_through_the_handle_the_mount_built() {
    // Three activations, three linear memories, one element: the panel writes
    // the delivered number through a handle it minted two activations earlier,
    // which is the per-activation memory rule in one assertion.
    let mut harness = mounted();
    let total = harness.page().marked_child(ROOT, TOTAL_MARKER);
    let button = harness.page().marked_child(ROOT, PRESS_MARKER);

    harness.call(gesture(PRESS_PORT, button));
    harness.call(delivery_on(TOTAL_PORT, &[], &[&total_body(4)], 0));

    assert_eq!(
        harness.page().marked_child(ROOT, TOTAL_MARKER),
        total,
        "no second view was built over the first",
    );
    assert_eq!(harness.page().text_of(total), "4");
}

#[test]
fn a_state_body_the_panel_cannot_read_fails_the_activation() {
    // A version skew across a deploy: a page left open over a release whose
    // `Panel` had another shape. Starting from a default would build a second
    // view over the one already on the page, so the activation is refused and a
    // reload is what repairs it.
    let mut harness = unmounted().seeding_state(r#"{"display":"nope"}"#);

    let error = harness.call_expecting_a_refusal(mount());
    match error {
        types::ReceiveError::ProcessingFailed(why) => assert!(
            why.contains(&format!("port {STATE_PORT:?}")) && why.contains("unreadable"),
            "the refusal names the port and says its body could not be read: {why}",
        ),
        other => panic!("an unreadable state body is a refusal, not a trap: {other:?}"),
    }
    assert!(
        harness.page().children(ROOT).is_empty(),
        "the refusal comes before the body runs, so nothing was drawn",
    );
}

#[test]
fn a_refused_state_write_fails_the_mount() {
    // The other way this activation's state is lost. The panel's state holds a
    // handle, so carrying on would mean the next activation building a second
    // view: the mount fails instead and the instance takes its error card.
    let mut harness = unmounted();
    harness
        .page()
        .publish_answer_on
        .insert(STATE_PORT.to_string(), ports::PublishError::QuotaExceeded);

    let error = harness.call_expecting_a_refusal(mount());
    match error {
        types::ReceiveError::ProcessingFailed(why) => assert!(
            why.contains(&format!("port {STATE_PORT:?}")),
            "the refusal names the port the write was refused on: {why}",
        ),
        other => panic!("a refused state write is a refusal, not a trap: {other:?}"),
    }
    // Not `published_on(STATE_PORT).is_empty()`: this test refused every
    // publish on that port, so the harness could record none whatever the guest
    // did. The transcript carries the attempt, so it is what says the store was
    // tried, on this port, once — and fails if the state write stops happening.
    let transcript = harness.transcript();
    let attempts = transcript
        .iter()
        .filter(|call| call.starts_with(&format!("ports.publish({STATE_PORT},")))
        .count();
    assert_eq!(
        attempts, 1,
        "the state is stored once and the refusal is not retried: {transcript:?}",
    );
}

#[test]
fn a_non_mount_activation_with_no_retained_state_is_refused() {
    // The wrong `push_depth`/`retain_depth` pair is the misconfiguration no
    // compiler here catches, and this refusal is its whole mechanical signal:
    // an activation that is not the mount and has no display handle cannot draw
    // and says so, rather than trapping or quietly doing nothing.
    let mut harness = unmounted();

    let error = harness.call_expecting_a_refusal(gesture(PRESS_PORT, ROOT));
    match error {
        types::ReceiveError::ProcessingFailed(why) => assert!(
            why.contains("no view"),
            "the refusal says the mount's state did not reach this activation: {why}",
        ),
        other => panic!("a missing view is a refusal, not a trap: {other:?}"),
    }
    assert_eq!(
        harness.page().published_on(CLICKS_PORT),
        vec!["{}"],
        "the press is published before the view is looked up",
    );
    // The host discards a refused activation's publish buffer, so this click
    // is lost — the cost of refusing rather than writing into a stale handle.
}

#[test]
fn a_delivery_with_no_retained_state_is_refused_too() {
    // The check runs before the windows are read, so an activation that would
    // have rendered a number is refused on the same terms as a press: there is
    // no element to write it to, and a delivery is not the activation that
    // builds one.
    let mut harness = unmounted();

    let four = total_body(4);
    let error = harness.call_expecting_a_refusal(delivery_on(TOTAL_PORT, &[], &[&four], 0));
    match error {
        types::ReceiveError::ProcessingFailed(why) => assert!(
            why.contains("no view"),
            "the refusal says the mount's state did not reach this activation: {why}",
        ),
        other => panic!("a missing view is a refusal, not a trap: {other:?}"),
    }
}

#[test]
fn a_mount_over_a_display_handle_from_an_earlier_mount_builds_a_new_one() {
    // Retained state from an earlier mount can include a display handle, but
    // the host element is cleared before each mount activation — that handle
    // names nothing. Trusting it would write every later total into nothing.
    let mut harness = unmounted().seeding_state(&state_body(4096));

    harness.call(mount());
    let total = harness.page().marked_child(ROOT, TOTAL_MARKER);
    assert_ne!(total, 4096, "the stale handle is not the one it kept");
    assert_eq!(
        harness.page().published_on(STATE_PORT),
        vec![state_body(total)],
        "the mount stores the display it built over the one it was handed",
    );

    harness.call(delivery_on(TOTAL_PORT, &[], &[&total_body(9)], 0));
    assert_eq!(harness.page().text_of(total), "9");
}
