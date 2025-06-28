mod applications;
mod audio;
mod notifications;
mod open_file;
mod quick_settings;
mod wallpaper;

use crate::mcp::ToolDefinition;

pub fn list_tools() -> Vec<ToolDefinition> {
    vec![
        notifications::get_tool_definition(),
        applications::get_tool_definition(),
        open_file::get_tool_definition(),
        wallpaper::get_tool_definition(),
        audio::get_volume_tool_definition(),
        audio::get_media_tool_definition(),
        quick_settings::get_tool_definition(),
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
        "set_wallpaper" => wallpaper::execute(arguments).await,
        "set_volume" => audio::execute_volume(arguments).await,
        "media_control" => audio::execute_media_control(arguments).await,
        "quick_settings" => quick_settings::execute(arguments).await,
        _ => Err(anyhow::anyhow!("Tool not found: {}", name)),
    }
}
