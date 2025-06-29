use anyhow::Result;

use crate::{mcp::ToolProvider, tool_params};

#[derive(Default)]
pub struct Notifications;

tool_params! {
    NotificationParams,
    required(summary: string, "Notification summary"),
    required(body: string, "Notification body")
}

impl ToolProvider for Notifications {
    const NAME: &'static str = "send_notification";
    const DESCRIPTION: &'static str = "Send a desktop notification";
    type Params = NotificationParams;

    async fn execute_with_params(&self, params: Self::Params) -> Result<serde_json::Value> {
        Self::execute_with_message(
            || send_notification(&params.summary, &params.body),
            format!("Notification sent: {}", params.summary),
        )
        .await
    }
}

async fn send_notification(summary: &str, body: &str) -> Result<()> {
    let proxy = ashpd::desktop::notification::NotificationProxy::new().await?;
    let notification = ashpd::desktop::notification::Notification::new(summary).body(body);

    proxy.add_notification("", notification).await?;
    Ok(())
}
