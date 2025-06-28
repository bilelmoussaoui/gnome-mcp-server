use crate::mcp::ToolDefinition;
use anyhow::Result;
use serde_json::json;
use zbus::Connection;

pub fn get_volume_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "set_volume".to_string(),
        description: "Control system volume and mute/unmute".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "volume": {
                    "type": "number",
                    "description": "Volume level (0-100, where 100 is maximum)",
                    "minimum": 0,
                    "maximum": 100
                },
                "mute": {
                    "type": "boolean",
                    "description": "Mute (true) or unmute (false) the system"
                },
                "relative": {
                    "type": "boolean",
                    "description": "If true, volume is relative change (+10, -5), if false, absolute level"
                }
            }
        }),
    }
}

pub fn get_media_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "media_control".to_string(),
        description: "Control media playback (play, pause, skip, etc.) via MPRIS".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["play", "pause", "play_pause", "stop", "next", "previous"],
                    "description": "Media control action to perform"
                },
                "player": {
                    "type": "string",
                    "description": "Specific player to control (optional, uses active player if not specified)"
                }
            },
            "required": ["action"]
        }),
    }
}

pub async fn execute_volume(arguments: &serde_json::Value) -> Result<serde_json::Value> {
    let volume = arguments.get("volume").and_then(|v| v.as_f64());
    let mute = arguments.get("mute").and_then(|v| v.as_bool());
    let relative = arguments
        .get("relative")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if volume.is_none() && mute.is_none() {
        return Ok(json!({
            "success": false,
            "error": "Must specify either volume or mute parameter"
        }));
    }

    match set_system_volume(volume, mute, relative).await {
        Ok(result) => Ok(json!({
            "success": true,
            "result": result
        })),
        Err(e) => Ok(json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

pub async fn execute_media_control(arguments: &serde_json::Value) -> Result<serde_json::Value> {
    let action = arguments
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing action parameter"))?;

    let player = arguments.get("player").and_then(|v| v.as_str());

    match control_media_playback(action, player).await {
        Ok(result) => Ok(json!({
            "success": true,
            "result": result
        })),
        Err(e) => Ok(json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

async fn set_system_volume(
    volume: Option<f64>,
    mute: Option<bool>,
    relative: bool,
) -> Result<String> {
    let mut results = Vec::new();

    if let Some(vol) = volume {
        let volume_str = if relative {
            if vol >= 0.0 {
                format!("{}%+", vol)
            } else {
                format!("{}%-", vol.abs())
            }
        } else {
            format!("{}%", vol)
        };

        // Try wpctl (WirePlumber control)
        let output = tokio::process::Command::new("wpctl")
            .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &volume_str])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "wpctl volume failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        results.push(format!("PipeWire: Volume set to {}", volume_str));
    }

    if let Some(should_mute) = mute {
        let mute_arg = if should_mute { "1" } else { "0" };
        let output = tokio::process::Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", mute_arg])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "wpctl mute failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        results.push(format!(
            "PipeWire: {}",
            if should_mute { "Muted" } else { "Unmuted" }
        ));
    }

    Ok(results.join(", "))
}

async fn control_media_playback(action: &str, player: Option<&str>) -> Result<String> {
    let connection = Connection::session().await?;

    // Find available MPRIS players
    let players = find_mpris_players(&connection).await?;

    if players.is_empty() {
        return Err(anyhow::anyhow!("No media players found"));
    }

    // Select target player
    let target_player = if let Some(player_name) = player {
        players
            .iter()
            .find(|p| p.to_lowercase().contains(&player_name.to_lowercase()))
            .ok_or_else(|| anyhow::anyhow!("Player '{}' not found", player_name))?
    } else {
        // Use the first available player
        &players[0]
    };

    // Connect to the MPRIS player
    let player_proxy = zbus::Proxy::new(
        &connection,
        target_player.to_owned(),
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )
    .await?;

    // Execute the requested action
    match action {
        "play" => {
            player_proxy.call_method("Play", &()).await?;
            Ok(format!("Started playback on {}", target_player))
        }
        "pause" => {
            player_proxy.call_method("Pause", &()).await?;
            Ok(format!("Paused playback on {}", target_player))
        }
        "play_pause" => {
            player_proxy.call_method("PlayPause", &()).await?;
            Ok(format!("Toggled playback on {}", target_player))
        }
        "stop" => {
            player_proxy.call_method("Stop", &()).await?;
            Ok(format!("Stopped playback on {}", target_player))
        }
        "next" => {
            player_proxy.call_method("Next", &()).await?;
            Ok(format!("Skipped to next track on {}", target_player))
        }
        "previous" => {
            player_proxy.call_method("Previous", &()).await?;
            Ok(format!("Skipped to previous track on {}", target_player))
        }
        _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
    }
}

async fn find_mpris_players(connection: &Connection) -> Result<Vec<String>> {
    let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await?;
    let names = dbus_proxy.list_names().await?;

    let players: Vec<String> = names
        .into_iter()
        .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
        .map(|n| n.to_string())
        .collect();

    Ok(players)
}
