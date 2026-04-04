use std::path::PathBuf;
use std::sync::Arc;

use minicode_agent_core::AgentTurnCallbacks;
use minicode_core::config::RuntimeConfig;
use minicode_core::types::{ChatMessage, ModelAdapter};
use minicode_permissions::{PermissionManager, PermissionPromptRequest, PermissionPromptResult};
use minicode_tool::{ToolRegistry, ToolResult};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub(crate) struct TranscriptEntry {
    pub(crate) kind: String,
    pub(crate) body: String,
}

pub(crate) struct PendingApproval {
    pub(crate) request: PermissionPromptRequest,
    pub(crate) responder: Option<oneshot::Sender<PermissionPromptResult>>,
    pub(crate) selected_index: usize,
    pub(crate) awaiting_feedback: bool,
    pub(crate) feedback: String,
}

pub(crate) enum TurnEvent {
    ToolStart {
        tool_name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_name: String,
        output: String,
        is_error: bool,
    },
    Assistant(String),
    Progress(String),
    Approval {
        request: PermissionPromptRequest,
        responder: oneshot::Sender<PermissionPromptResult>,
    },
    Done(Vec<ChatMessage>),
    ToolDone(ToolResult),
}

#[derive(Default)]
pub(crate) struct ScreenState {
    pub(crate) input: String,
    pub(crate) cursor_offset: usize,
    pub(crate) transcript: Vec<TranscriptEntry>,
    pub(crate) transcript_scroll_offset: usize,
    pub(crate) selected_slash_index: usize,
    pub(crate) status: Option<String>,
    pub(crate) active_tool: Option<String>,
    pub(crate) recent_tools: Vec<(String, bool)>,
    pub(crate) history: Vec<String>,
    pub(crate) history_index: usize,
    pub(crate) history_draft: String,
    pub(crate) is_busy: bool,
    pub(crate) message_count: usize,
    pub(crate) pending_approval: Option<PendingApproval>,
}

pub struct TuiAppArgs {
    pub runtime: Option<RuntimeConfig>,
    pub tools: Arc<ToolRegistry>,
    pub model: Arc<dyn ModelAdapter>,
    pub cwd: PathBuf,
    pub permissions: PermissionManager,
}

pub(crate) struct ChannelCallbacks {
    pub(crate) tx: mpsc::UnboundedSender<TurnEvent>,
}

impl AgentTurnCallbacks for ChannelCallbacks {
    fn on_tool_start(&mut self, tool_name: &str, input: &serde_json::Value) {
        let _ = self.tx.send(TurnEvent::ToolStart {
            tool_name: tool_name.to_string(),
            input: input.clone(),
        });
    }

    fn on_tool_result(&mut self, tool_name: &str, output: &str, is_error: bool) {
        let _ = self.tx.send(TurnEvent::ToolResult {
            tool_name: tool_name.to_string(),
            output: output.to_string(),
            is_error,
        });
    }

    fn on_assistant_message(&mut self, content: &str) {
        let _ = self.tx.send(TurnEvent::Assistant(content.to_string()));
    }

    fn on_progress_message(&mut self, content: &str) {
        let _ = self.tx.send(TurnEvent::Progress(content.to_string()));
    }
}
