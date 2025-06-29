use crate::mcp::ToolProvider;
use crate::tool_params;
use anyhow::Result;
use zbus::Connection;

#[derive(Default)]
pub struct Volume;

tool_params! {
    VolumeParams,
    optional(volume: f64, "Volume level (0-100, where 100 is maximum)"),
    optional(mute: bool, "Mute (true) or unmute (false) the system"),
    optional(relative: bool, "If true, volume is relative change (+10, -5), if false, absolute level"),
    optional(direction: string, "Direction for default step: 'up' or 'down' (uses config volume_step)")
}

impl ToolProvider for Volume {
    const NAME: &'static str = "set_volume";
    const DESCRIPTION: &'static str = "Control system volume and mute/unmute";
    type Params = VolumeParams;

    async fn execute_with_params(&self, params: Self::Params) -> Result<serde_json::Value> {
        let config = crate::config::CONFIG.get_audio_tool_config();

        if let Some(volume) = params.volume {
            Self::execute_with_result(|| {
                set_system_volume(volume, params.relative.unwrap_or(false))
            })
            .await
        } else if let Some(direction) = params.direction {
            let volume_change = match direction.as_str() {
                "up" => config.volume_step as f64,
                "down" => -(config.volume_step as f64),
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid direction: {}. Use 'up' or 'down'",
                        direction
                    ));
                }
            };
            Self::execute_with_result(|| set_system_volume(volume_change, true)).await
        } else if let Some(mute) = params.mute {
            Self::execute_with_result(|| set_system_mute(mute)).await
        } else {
            Err(anyhow::anyhow!(
                "Must specify either volume, direction, or mute parameter"
            ))
        }
    }
}

#[derive(Default)]
pub struct Media;

tool_params! {
    MediaParams,
    required(action: string, "Media control action to perform"),
    optional(player: string = "".to_string(), "Specific player to control (optional, uses active player if not specified)")
}

impl ToolProvider for Media {
    const NAME: &'static str = "media_control";
    const DESCRIPTION: &'static str = "Control media playback (play, pause, skip, etc.) via MPRIS";
    type Params = MediaParams;

    async fn execute_with_params(&self, params: Self::Params) -> Result<serde_json::Value> {
        let player_ref = if params.player.is_empty() {
            None
        } else {
            Some(params.player.as_str())
        };

        Self::execute_with_result(|| control_media_playback(&params.action, player_ref)).await
    }
}

async fn set_system_volume(volume: f64, relative: bool) -> Result<String> {
    let volume_str = if relative {
        if volume >= 0.0 {
            format!("{volume}%+")
        } else {
            format!("{}%-", volume.abs())
        }
    } else {
        format!("{volume}%")
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

    Ok(format!("PipeWire: Volume set to {volume_str}"))
}

async fn set_system_mute(mute: bool) -> Result<String> {
    let mute_arg = if mute { "1" } else { "0" };
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

    Ok(format!(
        "PipeWire: {}",
        if mute { "Muted" } else { "Unmuted" }
    ))
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
            Ok(format!("Started playback on {target_player}"))
        }
        "pause" => {
            player_proxy.call_method("Pause", &()).await?;
            Ok(format!("Paused playback on {target_player}"))
        }
        "play_pause" => {
            player_proxy.call_method("PlayPause", &()).await?;
            Ok(format!("Toggled playback on {target_player}"))
        }
        "stop" => {
            player_proxy.call_method("Stop", &()).await?;
            Ok(format!("Stopped playback on {target_player}"))
        }
        "next" => {
            player_proxy.call_method("Next", &()).await?;
            Ok(format!("Skipped to next track on {target_player}"))
        }
        "previous" => {
            player_proxy.call_method("Previous", &()).await?;
            Ok(format!("Skipped to previous track on {target_player}"))
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
