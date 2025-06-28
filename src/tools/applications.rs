use crate::mcp::ToolDefinition;
use anyhow::Result;
use gio::prelude::*;
use serde_json::json;

pub fn get_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "launch_application".to_owned(),
        description: "Launch an application by name or executable".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "app_name": {
                    "type": "string",
                    "description": "Application name (e.g., 'Firefox', 'Terminal')"
                }
            },
            "required": ["app_name"]
        }),
    }
}

pub async fn execute(arguments: &serde_json::Value) -> Result<serde_json::Value> {
    let app_name = arguments
        .get("app_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing app_name"))?;

    let app_infos = gio::AppInfo::all();
    let total_apps = app_infos.len();

    for app_info in app_infos {
        if !app_info.should_show() {
            continue;
        }

        let name = app_info.name().to_lowercase();
        let app_name_lower = app_name.to_lowercase();

        if name.contains(&app_name_lower) {
            // Try launching and return detailed info
            app_info.launch(&[], gio::AppLaunchContext::NONE)?;
            break;
        }
    }

    Err(anyhow::anyhow!(
        "App '{}' not found among {} total apps",
        app_name,
        total_apps
    ))
}
