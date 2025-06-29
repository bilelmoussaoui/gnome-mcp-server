use crate::mcp::ToolProvider;
use anyhow::Result;
use serde_json::json;
use zbus::Connection;

#[derive(Default)]
pub struct Notifications;

impl ToolProvider for Notifications {
    const NAME: &'static str = "send_notification";
    const DESCRIPTION: &'static str = "Send a desktop notification";

    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "Notification summary"
                },
                "body": {
                    "type": "string",
                    "description": "Notification body"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Notification timeout in milliseconds"
                }
            },
            "required": ["summary", "body"]
        })
    }

    async fn execute(&self, arguments: &serde_json::Value) -> Result<serde_json::Value> {
        let summary = arguments
            .get("summary")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing summary"))?;

        let body = arguments
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing body"))?;

        let timeout = arguments
            .get("timeout")
            .and_then(|v| v.as_i64())
            .unwrap_or(5000);

        send_notification(summary, body, timeout).await?;

        Ok(json!({
            "success": true,
            "result": format!("Notification sent: {}", summary)
        }))
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
