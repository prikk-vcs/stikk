//! The Queue view-model (design `FR-051`, `TU-01`; RFC 028 Handoff A §4 and §6).
//!
//! **Every word the Queue screen shows is decided here**, so a GUI says the same thing. The frontend
//! renders [`QueueView`]'s lines in order, and routes every one through `inert` (`C-T2a`), since names,
//! paths, messages and key ids are repository text.
//!
//! **One read.** At prikk ≥ 0.39 the count and the list come from the same `queue` report, so they
//! describe the same moment. Below 0.39 the list is unreported, and the view says so rather than show an
//! empty one (`C-T2c′`).

use std::path::Path;

use stikk_model::Result;
use stikk_prikk::{
    Prikk, QueueReport, QueueTarget, QueueThreshold, QueuedMessage, QueuedOperation, QueuedPatch,
    QueuedPath, ThresholdStatus,
};

/// The first prikk minor whose queued patches carry a message (prikk 0.42).
const QUEUED_MESSAGE_FROM_MINOR: u32 = 42;

/// The Queue screen, as lines in stikk's words (RFC 028 Handoff A §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueView {
    /// The heading: the count and the target, or `Nothing is queued.`.
    pub heading: String,
    /// The thresholds, when prikk reports them: prikk's numbers, then where the queue stands. Empty
    /// otherwise.
    pub thresholds: Vec<String>,
    /// Each queued patch, in prikk's order. Empty for an empty queue, and below 0.39.
    pub patches: Vec<QueuedPatchView>,
    /// The one line at the foot, when one applies: what this prikk does not report, or, below 0.39, that it
    /// does not list queued patches. `None` for an empty queue.
    pub foot: Option<String>,
}

/// One queued patch, as lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedPatchView {
    /// The patch id, in full.
    pub id: String,
    /// The message line: the message's first line (with `…` when it has more), or `(no message)`. `None`
    /// when this prikk does not report messages, which the foot says once.
    pub message: Option<String>,
    /// One line per operation, or per path of an operation that touches several.
    pub operations: Vec<String>,
}

/// Build the Queue view (design `FR-051`; RFC 028 decision 2, as revised at prikk 0.42.0).
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises.
pub fn queue_view(prikk: &impl Prikk, repo: &Path) -> Result<QueueView> {
    let prikk_minor = prikk.handshake()?.version.minor;
    Ok(view(prikk.queue(repo)?, prikk_minor))
}

fn view(report: QueueReport, prikk_minor: u32) -> QueueView {
    match report {
        QueueReport::Unreported { count: 0, .. } => nothing_queued(),
        QueueReport::Listed(queue) if queue.count == 0 => nothing_queued(),
        // Below 0.39: the heading, then one line saying the list is not reported — never an empty list.
        QueueReport::Unreported { count, target } => QueueView {
            heading: match target {
                Some(target) => format!("{count} patch(es) queued for {target}"),
                // The prose form does not tell prikk's missing and malformed sentinels apart.
                None => format!("{count} patch(es) queued; prikk reports no target ref"),
            },
            thresholds: Vec::new(),
            patches: Vec::new(),
            foot: Some(format!(
                "prikk 0.{prikk_minor} does not list queued patches."
            )),
        },
        QueueReport::Listed(queue) => QueueView {
            heading: heading(queue.count, &queue.target),
            thresholds: queue.threshold.map(thresholds).unwrap_or_default(),
            patches: queue.patches.into_iter().map(patch).collect(),
            foot: Some(if prikk_minor < QUEUED_MESSAGE_FROM_MINOR {
                format!(
                    "prikk 0.{prikk_minor} does not report a queued patch's message or content; both \
                     appear in History once it is sealed."
                )
            } else {
                "A queued patch's content is not shown here; stikk's Patch detail view is not built yet."
                    .to_string()
            }),
        },
    }
}

/// An empty queue: the heading, and nothing else below it.
fn nothing_queued() -> QueueView {
    QueueView {
        heading: "Nothing is queued.".to_string(),
        thresholds: Vec::new(),
        patches: Vec::new(),
        foot: None,
    }
}

fn heading(count: u64, target: &QueueTarget) -> String {
    match target {
        QueueTarget::Ref(target) => format!("{count} patch(es) queued for {target}"),
        QueueTarget::MissingMetadata => {
            format!("{count} patch(es) queued; prikk reports the target ref's metadata as missing")
        }
        QueueTarget::MalformedMetadata => format!(
            "{count} patch(es) queued; prikk reports the target ref's metadata as malformed"
        ),
        // prikk 0.42.0 writes both target fields `null` only for an empty queue, which never reaches here.
        // Held rather than refused, in the words History's tier uses for the same fact.
        QueueTarget::NotReported => {
            format!("{count} patch(es) queued; prikk reports no target ref")
        }
    }
}

/// prikk's numbers, then where the queue stands. Nothing about what prikk does at the limit: that is not
/// measured here.
fn thresholds(threshold: QueueThreshold) -> Vec<String> {
    vec![
        format!(
            "warning at {} · hard limit at {}",
            threshold.warn, threshold.hard_limit
        ),
        match threshold.status {
            ThresholdStatus::None => "below the warning threshold",
            ThresholdStatus::Warn => "at or above the warning threshold",
            ThresholdStatus::HardLimit => "at or above the hard limit",
        }
        .to_string(),
    ]
}

fn patch(patch: QueuedPatch) -> QueuedPatchView {
    QueuedPatchView {
        message: message_line(&patch.message),
        operations: patch.operations.iter().flat_map(operation_lines).collect(),
        id: patch.patch_id,
    }
}

/// The message's line: its first line, with `…` marking the rest; `(no message)` for prikk's `null`; and
/// nothing when this prikk does not report messages — **never `(no message)` for that** (`C-T2c′`).
pub(crate) fn message_line(message: &QueuedMessage) -> Option<String> {
    match message {
        QueuedMessage::NotReported => None,
        QueuedMessage::None => Some("(no message)".to_string()),
        QueuedMessage::Text(text) => {
            let mut lines = text.lines();
            let first = lines.next().unwrap_or_default();
            Some(if lines.next().is_some() {
                format!("{first}…")
            } else {
                first.to_string()
            })
        }
    }
}

/// An operation's lines. A rename from one path to another is one line naming both and its asserting key;
/// anything else is one line per path, the kind word verbatim so a kind stikk does not know still renders.
fn operation_lines(operation: &QueuedOperation) -> Vec<String> {
    let kind = &operation.kind;
    if kind == "rename-path"
        && let (Some(key), [QueuedPath::Path(from), QueuedPath::Path(to)]) = (
            operation.author_key_id.as_deref(),
            operation.paths.as_slice(),
        )
    {
        return vec![format!("rename-path {from} → {to} · asserted by {key}")];
    }
    let asserted = operation
        .author_key_id
        .as_deref()
        .map(|key| format!(" · asserted by {key}"))
        .unwrap_or_default();
    if operation.paths.is_empty() {
        return vec![format!("{kind}{asserted}")];
    }
    operation
        .paths
        .iter()
        .map(|path| match path {
            QueuedPath::Path(path) => format!("{kind} {path}{asserted}"),
            QueuedPath::UnresolvedNode(node) => {
                format!("{kind} unresolved node {node} (no longer in the baseline){asserted}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
