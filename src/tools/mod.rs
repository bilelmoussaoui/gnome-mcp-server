pub mod applications;
pub mod notifications;
pub mod open_file;

use crate::mcp::ToolDefinition;

pub fn list_tools() -> Vec<ToolDefinition> {
    vec![
        notifications::get_tool_definition(),
        applications::get_tool_definition(),
        open_file::get_tool_definition(),
    ]
}

pub async fn execute_tool(
    name: &str,
    arguments: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    match name {
        "send_notification" => notifications::execute(arguments).await,
        "launch_application" => applications::execute(arguments).await,
        "open_file" => open_file::execute(arguments).await,
        _ => Err(anyhow::anyhow!("Tool not found: {}", name)),
    }
}
