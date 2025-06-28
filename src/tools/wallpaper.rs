use crate::mcp::ToolDefinition;
use anyhow::Result;
use serde_json::json;
use zbus::Connection;

pub fn get_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "set_wallpaper".to_owned(),
        description: "Set the desktop wallpaper from a local file path".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "image_path": {
                    "type": "string",
                    "description": "Full path to the image file (e.g., '/tmp/wallpaper.jpg', '/home/user/Pictures/photo.png')"
                }
            },
            "required": ["image_path"]
        }),
    }
}

pub async fn execute(arguments: &serde_json::Value) -> Result<serde_json::Value> {
    let image_path = arguments
        .get("image_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing image_path"))?;
    // Validate file exists and is an image
    validate_image_file(image_path)?;

    // Convert to file:// URI format
    let image_uri = if image_path.starts_with("file://") {
        image_path.to_string()
    } else {
        format!(
            "file://{}",
            std::path::Path::new(image_path).canonicalize()?.display()
        )
    };

    match set_wallpaper(&image_uri).await {
        Ok(result) => Ok(json!({
            "success": true,
            "result": format!("Wallpaper set to: {}", image_path),
            "details": result
        })),
        Err(e) => Ok(json!({
            "success": false,
            "error": e.to_string(),
            "debug": format!("Failed to set wallpaper: {}", image_path)
        })),
    }
}

fn validate_image_file(image_path: &str) -> Result<()> {
    let path = std::path::Path::new(image_path);

    // Check if file exists
    if !path.exists() {
        return Err(anyhow::anyhow!("Image file does not exist: {}", image_path));
    }

    // Check if it's a file (not directory)
    if !path.is_file() {
        return Err(anyhow::anyhow!("Path is not a file: {}", image_path));
    }

    // Check file extension
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        let ext_lower = extension.to_lowercase();
        if !["jpg", "jpeg", "png"].contains(&ext_lower.as_str()) {
            return Err(anyhow::anyhow!("Unsupported image format: {}", extension));
        }
    } else {
        return Err(anyhow::anyhow!("No file extension found"));
    }

    Ok(())
}

async fn set_wallpaper(image_uri: &str) -> Result<String> {
    let connection = Connection::session().await?;

    // Use XDG Desktop Portal Wallpaper interface
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Wallpaper",
    )
    .await?;

    // The Wallpaper portal SetWallpaperURI method
    // Parameters: (parent_window, uri, options)
    let parent_window = ""; // Empty string for no parent window
    let options = std::collections::HashMap::<String, zbus::zvariant::Value>::new();

    match proxy
        .call_method("SetWallpaperURI", &(parent_window, image_uri, options))
        .await
    {
        Ok(_) => {
            // The portal returns a request handle, but for wallpaper it should be immediate
            Ok("Set via XDG Desktop Portal".to_string())
        }
        Err(e) => Err(anyhow::anyhow!("Wallpaper portal call failed: {}", e)),
    }
}
