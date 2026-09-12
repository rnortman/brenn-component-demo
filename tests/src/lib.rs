//! Host-side tests: both components, driven against the real hosts brenn ships.
//!
//! The two counter suites assert one artifact, two hostings, indistinguishable:
//! each reduces its host's output to `(port, body)` pairs and both compare
//! against [`EXPECTED`].

#[cfg(test)]
mod counter_backend;
#[cfg(test)]
mod counter_page;
#[cfg(test)]
mod panel;

/// The counter's input port carrying clicks.
pub const CLICKS_PORT: &str = "clicks";

/// The counter's `io` port: what it publishes, and what it reads its own last
/// publish back from.
pub const TOTAL_PORT: &str = "total";

/// What the `total` port's window carries on one activation.
///
/// The counter reads its own last publish back out of this window, so which
/// shape it arrives in is the whole of its memory model: absent before anything
/// published, present but empty when the window exists and holds nothing, and
/// carrying the retained message as context or as news depending on whether the
/// activation that sees it is the one it arrived on.
pub enum TotalWindow {
    /// No `total` window on the activation at all.
    Absent,
    /// A `total` window carrying nothing — not the same as no window.
    Empty,
    /// The retained total, already seen: window context.
    Context(u64),
    /// The retained total, arriving on this activation: news.
    New(u64),
}

/// One activation of the shared script, described in terms neither host owns.
pub struct Step {
    pub new_clicks: usize,
    pub total: TotalWindow,
}

/// The script both counter suites drive, in order.
///
/// The mount activation first — every instance is owed one when it is put into
/// service, over whatever its channels happen to hold, which at a cold start is
/// nothing. Then a first click activation with no retained total, then the
/// arithmetic on a running total offered as context, then the same offered as
/// news — the guest must treat both identically — then an empty window, which
/// is what a port bound to a channel nothing has published on delivers. Last,
/// a retained total with no new clicks over it: the shape a reload that
/// replaces the consumer delivers, which republishes the total unchanged
/// rather than going quiet, so a panel that came up after it is told a number.
pub const SCRIPT: [Step; 6] = [
    Step {
        new_clicks: 0,
        total: TotalWindow::Empty,
    },
    Step {
        new_clicks: 2,
        total: TotalWindow::Absent,
    },
    Step {
        new_clicks: 2,
        total: TotalWindow::Context(3),
    },
    Step {
        new_clicks: 1,
        total: TotalWindow::New(5),
    },
    Step {
        new_clicks: 3,
        total: TotalWindow::Empty,
    },
    Step {
        new_clicks: 0,
        total: TotalWindow::Context(3),
    },
];

/// The index of the script's first click activation — two clicks, no retained
/// total yet.
///
/// Naming it keeps a step inserted ahead of it from silently retargeting
/// the single-activation tests that index [`EXPECTED`] by it.
pub const COLD_CLICK: usize = 1;

/// What the script must publish, in order, on whichever host ran it: `(port,
/// body)`.
///
/// The bodies are literal because that is what crosses the bus. A component
/// that renamed the field or started publishing a bare number would still
/// satisfy a structural comparison and would break every consumer of the
/// doctype.
pub const EXPECTED: [(&str, &str); 6] = [
    (TOTAL_PORT, r#"{"total":0}"#),
    (TOTAL_PORT, r#"{"total":2}"#),
    (TOTAL_PORT, r#"{"total":5}"#),
    (TOTAL_PORT, r#"{"total":6}"#),
    (TOTAL_PORT, r#"{"total":3}"#),
    (TOTAL_PORT, r#"{"total":3}"#),
];

/// A body that is not a `brenn.demo.total@1`: the doctype's field, misspelled.
///
/// What a deployment that bound `total` to a channel carrying something else
/// delivers. The component refuses the activation rather than reading it as
/// zero, and both counter suites and the panel hold it to that.
pub const WRONG_DOCTYPE_BODY: &str = r#"{"tot":1}"#;

/// Builds the JSON body for a `total` publish.
pub fn total_body(total: u64) -> String {
    format!(r#"{{"total":{total}}}"#)
}

/// An artifact path from the build.
///
/// An out-of-tree crate cannot walk to a component: it sits wherever its own
/// repository puts it, and under Bazel it sits in the runfiles. The BUILD file
/// names the artifact through `$(rootpath)` and the test reads it back here.
pub fn artifact(var: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var(var).unwrap_or_else(|e| panic!("the build names the artifact in {var}: {e}")),
    )
}
