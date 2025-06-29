use crate::mcp::macros::register_providers;
use crate::mcp::{Request, ResourceProvider, Response, ToolProvider};
use anyhow::Result;
use serde_json::json;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

register_providers! {
    resources: [
        crate::resources::system_info::SystemInfo,
        crate::resources::applications::Applications,
        crate::resources::calendar::Calendar,
        crate::resources::tasks::Tasks,
        crate::resources::audio::Audio,
    ],
    tools: [
        crate::tools::notifications::Notifications,
        crate::tools::applications::Applications,
        crate::tools::open_file::OpenFile,
        crate::tools::wallpaper::Wallpaper,
        crate::tools::audio::Volume,
        crate::tools::audio::Media,
        crate::tools::quick_settings::QuickSettings,
        crate::tools::screenshot::Screenshot,
    ]
}

pub struct Server;

impl Server {
    pub async fn run() -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF
            }

            if let Ok(request) = serde_json::from_str::<Request>(&line) {
                let response = Self::handle_request(request).await?;
                let response_json = serde_json::to_string(&response)?;
                stdout.write_all(response_json.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }

        Ok(())
    }

    async fn handle_request(request: Request) -> Result<Response> {
        let result = match request.method.as_str() {
            "initialize" => Self::handle_initialize().await,
            "resources/list" => Self::handle_list_resources().await,
            "resources/read" => Self::handle_read_resource(&request).await,
            "tools/list" => Self::handle_list_tools().await,
            "tools/call" => Self::handle_call_tool(&request).await,
            _ => json!({"error": "Method not found"}),
        };

        Ok(Response {
            jsonrpc: "2.0".to_owned(),
            id: request.id,
            result,
        })
    }

    async fn handle_initialize() -> serde_json::Value {
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "resources": {},
                "tools": {}
            },
            "serverInfo": {
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION")
            }
        })
    }

    async fn handle_list_tools() -> serde_json::Value {
        let tools = list_tools();
        json!({
            "tools": tools
        })
    }

    async fn handle_list_resources() -> serde_json::Value {
        let resources = list_resources();
        json!({
            "resources": resources
        })
    }

    async fn handle_call_tool(request: &Request) -> serde_json::Value {
        if let Some(params) = &request.params {
            if let (Some(name), Some(arguments)) = (
                params.get("name").and_then(|n| n.as_str()),
                params.get("arguments"),
            ) {
                match execute_tool(name, arguments).await {
                    Ok(result) => json!({
                        "content": [
                            {
                                "type": "text",
                                "text": result.to_string()
                            }
                        ]
                    }),
                    Err(e) => json!({"error": format!("Tool execution failed: {}", e)}),
                }
            } else {
                json!({"error": "Missing tool name or arguments"})
            }
        } else {
            json!({"error": "Missing parameters"})
        }
    }

    async fn handle_read_resource(request: &Request) -> serde_json::Value {
        if let Some(params) = &request.params {
            if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                match resource_for_uri(uri).await {
                    Ok(content) => json!({
                        "contents": [{
                            "uri": content.uri,
                            "mimeType": content.mime_type,
                            "text": content.text
                        }]
                    }),
                    Err(e) => {
                        json!({
                            "contents": [{
                                "uri": uri,
                                "mimeType": "application/json",
                                "text": json!({
                                    "error": format!("Failed to read resource: {}", e),
                                    "uri": uri,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                }).to_string()
                            }]
                        })
                    }
                }
            } else {
                json!({
                    "contents": [{
                        "uri": "unknown",
                        "mimeType": "application/json",
                        "text": json!({"error": "Missing uri parameter"}).to_string()
                    }]
                })
            }
        } else {
            json!({
                "contents": [{
                    "uri": "unknown",
                    "mimeType": "application/json",
                    "text": json!({"error": "Missing parameters"}).to_string()
                }]
            })
        }
    }
}
