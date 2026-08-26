use serde::Serialize;
use tauri::ipc::Channel;
use tokio::sync::mpsc;

use crate::process::{OutputLine, OutputStream};

/// Streamed progress for long-running tool invocations.
///
/// Channels are used instead of the global event system because they preserve ordering and are
/// the documented choice for streaming child-process output.
#[derive(Debug, Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum ProgressEvent {
    Started { label: String },
    Output { stream: OutputStream, text: String },
    Finished { success: bool },
}

/// Bridges the internal line stream onto a frontend channel.
///
/// Returns the sender to hand to the process layer; forwarding stops when it is dropped.
pub fn forward_to_channel(
    channel: Channel<ProgressEvent>,
    label: impl Into<String>,
) -> mpsc::UnboundedSender<OutputLine> {
    let (sender, mut receiver) = mpsc::unbounded_channel::<OutputLine>();

    let _ = channel.send(ProgressEvent::Started {
        label: label.into(),
    });

    tokio::spawn(async move {
        while let Some(line) = receiver.recv().await {
            if channel
                .send(ProgressEvent::Output {
                    stream: line.stream,
                    text: line.text,
                })
                .is_err()
            {
                break;
            }
        }
    });

    sender
}

pub fn finish(channel: &Channel<ProgressEvent>, success: bool) {
    let _ = channel.send(ProgressEvent::Finished { success });
}
