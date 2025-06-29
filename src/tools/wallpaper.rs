use crate::mcp::ToolProvider;
use crate::tool_params;
use anyhow::Result;
use zbus::Connection;

#[derive(Default)]
pub struct Wallpaper;

tool_params! {
    WallpaperParams,
    required(image_path: string, "Full path to the image file (e.g., '/tmp/wallpaper.jpg', '/home/user/Pictures/photo.png')")
}

impl ToolProvider for Wallpaper {
    const NAME: &'static str = "set_wallpaper";
    const DESCRIPTION: &'static str = "Set the desktop wallpaper from a local file path";
    type Params = WallpaperParams;

    async fn execute_with_params(&self, params: Self::Params) -> Result<serde_json::Value> {
        // Validate file exists and is an image
        validate_image_file(&params.image_path)?;

        // Convert to file:// URI format
        let image_uri = if params.image_path.starts_with("file://") {
            params.image_path.clone()
        } else {
            format!(
                "file://{}",
                std::path::Path::new(&params.image_path)
                    .canonicalize()?
                    .display()
            )
        };

        Self::execute_with_message(
            || set_wallpaper(&image_uri),
            format!("Wallpaper set to: {}", params.image_path),
        )
        .await
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

async fn set_wallpaper(image_uri: &str) -> Result<()> {
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

    proxy
        .call_method("SetWallpaperURI", &(parent_window, image_uri, options))
        .await?;
    Ok(())
}
