//! Maps the real `agent_client_protocol::schema::v1::SessionUpdate` enum
//! (agent -> client `session/update` notifications) onto Runyard's own
//! stable `AcpEvent` shape. This is the ONE place that needs to change if
//! the upstream crate's schema changes - everything else in Runyard depends
//! on `AcpEvent`, not on the wire schema directly.
use agent_client_protocol::schema::v1::SessionUpdate;
use serde_json::json;

use crate::events::AcpEvent;

pub fn map_session_update(connection_id: &str, session_id: &str, update: &SessionUpdate) -> AcpEvent {
    let connection_id = connection_id.to_string();
    let session_id = session_id.to_string();

    match update {
        SessionUpdate::AgentMessageChunk(chunk) => AcpEvent::AgentMessageChunk {
            connection_id,
            session_id,
            text: content_block_text(&chunk.content),
        },
        SessionUpdate::UserMessageChunk(chunk) => AcpEvent::UserMessageChunk {
            connection_id,
            session_id,
            text: content_block_text(&chunk.content),
        },
        SessionUpdate::AgentThoughtChunk(chunk) => AcpEvent::ThoughtChunk {
            connection_id,
            session_id,
            text: content_block_text(&chunk.content),
        },
        SessionUpdate::ToolCall(call) => AcpEvent::ToolCall {
            connection_id,
            session_id,
            tool_call_id: call.tool_call_id.to_string(),
            name: call.title.clone(),
            arguments: serde_json::to_value(call).unwrap_or(json!({})),
        },
        SessionUpdate::ToolCallUpdate(update) => AcpEvent::ToolCallUpdate {
            connection_id,
            session_id,
            tool_call_id: update.tool_call_id.to_string(),
            status: format!("{:?}", update.fields.status),
            content: serde_json::to_value(update).unwrap_or(json!({})),
        },
        SessionUpdate::Plan(plan) => AcpEvent::PlanUpdate {
            connection_id,
            session_id,
            plan: serde_json::to_value(plan).unwrap_or(json!({})),
        },
        SessionUpdate::AvailableCommandsUpdate(cmds) => AcpEvent::AvailableCommandsUpdate {
            connection_id,
            session_id,
            commands: serde_json::to_value(cmds).unwrap_or(json!({})),
        },
        SessionUpdate::CurrentModeUpdate(info) => AcpEvent::SessionInfoUpdate {
            connection_id,
            session_id,
            info: serde_json::to_value(info).unwrap_or(json!({})),
        },
        #[allow(unreachable_patterns)]
        other => AcpEvent::SessionInfoUpdate {
            connection_id,
            session_id,
            info: serde_json::to_value(other).unwrap_or(json!({})),
        },
    }
}

fn content_block_text(block: &agent_client_protocol::schema::v1::ContentBlock) -> String {
    match block {
        agent_client_protocol::schema::v1::ContentBlock::Text(t) => t.text.clone(),
        other => format!("{other:?}"),
    }
}
