use crate::mcp::ToolDefinition;
use anyhow::Result;
use serde_json::json;
use zbus::Connection;

pub fn get_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "send_notification".to_owned(),
        description: "Send a desktop notification".to_owned(),
        input_schema: json!({
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
        }),
    }
}

pub async fn execute(arguments: &serde_json::Value) -> Result<serde_json::Value> {
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
