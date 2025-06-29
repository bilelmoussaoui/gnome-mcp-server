use crate::mcp::ToolProvider;
use crate::tool_params;
use anyhow::Result;
use ashpd::desktop::screenshot::Screenshot as ScreenshotPortal;

#[derive(Default)]
pub struct Screenshot;

tool_params! {
    ScreenshotParams,
    optional(interactive: bool, "Show interactive screenshot dialog for area selection"),
}

impl ToolProvider for Screenshot {
    const NAME: &'static str = "take_screenshot";
    const DESCRIPTION: &'static str = "Take a screenshot using the desktop portal";
    type Params = ScreenshotParams;

    async fn execute_with_params(&self, params: Self::Params) -> Result<serde_json::Value> {
        let config = crate::config::CONFIG.get_screenshot_config();

        let interactive = params.interactive.unwrap_or(config.interactive);

        Self::execute_with_result(|| take_screenshot_portal(interactive)).await
    }
}

async fn take_screenshot_portal(interactive: bool) -> Result<String> {
    match ScreenshotPortal::request()
        .interactive(interactive)
        .send()
        .await?
        .response()
    {
        Ok(response) => {
            let uri = response.uri();
            if interactive {
                Ok(format!(
                    "Interactive screenshot completed. File saved to: {uri}"
                ))
            } else {
                Ok(format!("Screenshot taken. File saved to: {uri}"))
            }
        }
        Err(error) => Err(anyhow::anyhow!(
            "Screenshot was cancelled or failed {}",
            error
        )),
    }
}
