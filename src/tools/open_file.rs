use crate::mcp::ToolProvider;
use anyhow::Result;
use gio::prelude::*;
use serde_json::json;

#[derive(Default)]
pub struct OpenFile;

impl ToolProvider for OpenFile {
    const NAME: &'static str = "open_file";
    const DESCRIPTION: &'static str = "Open a file or URL with the default application";

    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path or URL to open (e.g., '/home/user/document.pdf', 'https://example.com')"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, arguments: &serde_json::Value) -> Result<serde_json::Value> {
        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;

        match open_file(path).await {
            Ok(result) => Ok(json!({
                "success": true,
                "result": format!("Opened: {}", path),
                "debug": result
            })),
            Err(e) => Ok(json!({
                "success": false,
                "error": e.to_string(),
                "debug": format!("Failed to open: {}", path)
            })),
        }
    }
}

async fn open_file(path: &str) -> Result<String> {
    // Method 1: Use GIO AppInfo to launch with default app
    if let Ok(result) = try_gio_launch(path).await {
        return Ok(result);
    }

    // Method 2: Use xdg-open (universal)
    if let Ok(result) = try_xdg_open(path).await {
        return Ok(result);
    }

    Err(anyhow::anyhow!("All open methods failed for: {}", path))
}

async fn try_gio_launch(path: &str) -> Result<String> {
    // Convert path to GFile
    let file = if path.starts_with("http://")
        || path.starts_with("https://")
        || path.starts_with("file://")
    {
        gio::File::for_uri(path)
    } else {
        gio::File::for_path(path)
    };

    // Get default app for this file type
    let file_info = file.query_info(
        "standard::content-type",
        gio::FileQueryInfoFlags::NONE,
        gio::Cancellable::NONE,
    )?;

    if let Some(content_type) = file_info.content_type() {
        if let Some(app_info) = gio::AppInfo::default_for_type(&content_type, false) {
            match app_info.launch(&[file], Option::<&gio::AppLaunchContext>::None) {
                Ok(_) => {
                    return Ok(format!("Opened with {} via GIO", app_info.name()));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("GIO launch failed: {}", e));
                }
            }
        }
    }

    Err(anyhow::anyhow!("No default app found via GIO"))
}

async fn try_xdg_open(path: &str) -> Result<String> {
    match std::process::Command::new("xdg-open").arg(path).spawn() {
        Ok(_) => Ok("Opened with xdg-open".to_owned()),
        Err(e) => Err(anyhow::anyhow!("xdg-open failed: {}", e)),
    }
}
