use crate::mcp::{ToolParams, ToolProvider};
use crate::tool_params;
use anyhow::Result;
use zbus::Connection;

#[derive(Default)]
pub struct Notifications;

tool_params! {
    NotificationParams,
    required(summary: string, "Notification summary"),
    required(body: string, "Notification body");
    optional(timeout: i64 = 5000, "Notification timeout in milliseconds")
}

impl ToolProvider for Notifications {
    const NAME: &'static str = "send_notification";
    const DESCRIPTION: &'static str = "Send a desktop notification";

    fn input_schema() -> serde_json::Value {
        NotificationParams::input_schema()
    }

    async fn execute(&self, arguments: &serde_json::Value) -> Result<serde_json::Value> {
        let params = NotificationParams::extract_params(arguments)?;

        Self::execute_with_message(
            || send_notification(&params.summary, &params.body, params.timeout),
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
