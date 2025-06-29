use crate::mcp::ToolProvider;
use crate::tool_params;
use anyhow::Result;
use gio::prelude::*;

#[derive(Default)]
pub struct Applications;

tool_params! {
    ApplicationParams,
    required(app_name: string, "Application name (e.g., 'Firefox', 'Terminal')")
}

impl ToolProvider for Applications {
    const NAME: &'static str = "launch_application";
    const DESCRIPTION: &'static str = "Launch an application by name or executable";
    type Params = ApplicationParams;

    async fn execute_with_params(&self, params: Self::Params) -> Result<serde_json::Value> {
        Self::execute_with_message(
            || launch_application(&params.app_name),
            format!("Successfully launched application: {}", params.app_name),
        )
        .await
    }
}

async fn launch_application(app_name: &str) -> Result<()> {
    let app_infos = gio::AppInfo::all();
    let total_apps = app_infos.len();

    for app_info in app_infos {
        if !app_info.should_show() {
            continue;
        }

        let name = app_info.name().to_lowercase();
        let app_name_lower = app_name.to_lowercase();

        if name.contains(&app_name_lower) {
            app_info.launch(&[], gio::AppLaunchContext::NONE)?;
            return Ok(());
        }
    }

    Err(anyhow::anyhow!(
        "App '{}' not found among {} total apps",
        app_name,
        total_apps
    ))
}
