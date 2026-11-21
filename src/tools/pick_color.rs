use anyhow::Result;

use crate::mcp::{ToolParams, ToolProvider};

#[derive(Default)]
pub struct PickColor;

/// The tool takes no parameters.
#[derive(Debug, Default)]
pub struct PickColorParams;

impl ToolParams for PickColorParams {
    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn extract_params(_arguments: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(Self)
    }
}

impl ToolProvider for PickColor {
    const NAME: &'static str = "pick_color";
    const DESCRIPTION: &'static str = "Pick a color from the screen using the desktop portal color picker. The user is prompted to click on a pixel to capture its color. Returns the color as a hex string, RGB values in the 0-255 range and normalized RGB values in the 0.0-1.0 range";
    type Params = PickColorParams;

    async fn execute_with_params(&self, _params: Self::Params) -> Result<serde_json::Value> {
        Self::execute_with_result(pick_color).await
    }
}

async fn pick_color() -> Result<serde_json::Value> {
    let color = ashpd::desktop::Color::pick()
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to request color picking: {}", e))?
        .response()
        .map_err(|e| anyhow::anyhow!("Color picking was cancelled or failed: {}", e))?;

    let red = color.red();
    let green = color.green();
    let blue = color.blue();

    // The portal returns normalized values in the 0.0-1.0 range
    let to_u8 = |channel: f64| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
    let (red_u8, green_u8, blue_u8) = (to_u8(red), to_u8(green), to_u8(blue));

    Ok(serde_json::json!({
        "hex": format!("#{red_u8:02X}{green_u8:02X}{blue_u8:02X}"),
        "rgb": [red_u8, green_u8, blue_u8],
        "rgb_normalized": [red, green, blue],
    }))
}
