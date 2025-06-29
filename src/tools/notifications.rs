use crate::mcp::ToolProvider;
use crate::tool_params;
use anyhow::Result;
use zbus::Connection;

#[derive(Default)]
pub struct Notifications;

tool_params! {
    NotificationParams,
    required(summary: string, "Notification summary"),
    required(body: string, "Notification body"),
    optional(timeout: i64, "Notification timeout in milliseconds")
}

impl ToolProvider for Notifications {
    const NAME: &'static str = "send_notification";
    const DESCRIPTION: &'static str = "Send a desktop notification";
    type Params = NotificationParams;

    async fn execute_with_params(&self, params: Self::Params) -> Result<serde_json::Value> {
        let config = crate::config::CONFIG.get_notifications_config();
        let timeout = params.timeout.unwrap_or(config.default_timeout as i64);

        Self::execute_with_message(
            || send_notification(&params.summary, &params.body, timeout),
            format!("Notification sent: {}", params.summary),
        )
        .await
    }
}

async fn send_notification(summary: &str, body: &str, timeout: i64) -> Result<()> {
    let connection = Connection::session().await?;

    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .await?;

    proxy
        .call_method(
            "Notify",
            &(
                env!("CARGO_PKG_NAME"),
                0u32, // replaces_id
                "",   // app_icon
                summary,
                body,
                Vec::<String>::new(), // actions
                std::collections::HashMap::<String, zbus::zvariant::Value>::new(), // hints
                timeout as i32,
            ),
        )
        .await?;

    Ok(())
}
