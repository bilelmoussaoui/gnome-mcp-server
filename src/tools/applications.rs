use crate::mcp::ToolProvider;
use anyhow::Result;
use gio::prelude::*;
use serde_json::json;

#[derive(Default)]
pub struct Applications;

impl ToolProvider for Applications {
    const NAME: &'static str = "launch_application";
    const DESCRIPTION: &'static str = "Launch an application by name or executable";

    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "app_name": {
                    "type": "string",
                    "description": "Application name (e.g., 'Firefox', 'Terminal')"
                }
            },
            "required": ["app_name"]
        })
    }

    async fn execute(&self, arguments: &serde_json::Value) -> Result<serde_json::Value> {
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
}
