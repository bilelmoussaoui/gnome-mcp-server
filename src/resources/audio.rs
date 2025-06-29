use crate::mcp::{ResourceContent, ResourceProvider};
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use zbus::Connection;

#[derive(Default)]
pub struct Audio;

impl ResourceProvider for Audio {
    const URI: &'static str = "gnome://audio/status";
    const NAME: &'static str = "Audio Status";
    const DESCRIPTION: &'static str =
        "Current system volume, mute state, and media playback status";

    async fn get_content(&self) -> Result<ResourceContent> {
        let audio_status = get_audio_status().await?;

        Ok(ResourceContent {
            uri: Self::URI,
            mime_type: Self::MIME_TYPE,
            text: audio_status.to_string(),
        })
    }
}

async fn get_audio_status() -> Result<serde_json::Value> {
    let connection = Connection::session().await?;

    let mut status = json!({
        "volume": {},
        "media": {}
    });

    // Get volume status
    if let Ok(volume_info) = get_volume_status().await {
        status["volume"] = volume_info;
    }

    // Get media status
    if let Ok(media_info) = get_media_status(&connection).await {
        status["media"] = media_info;
    }

    Ok(status)
}

async fn get_volume_status() -> Result<serde_json::Value> {
    let output = tokio::process::Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("wpctl get-volume failed"));
    }

    let volume_output = String::from_utf8_lossy(&output.stdout);

    // Parse output like "Volume: 0.65 [MUTED]"
    let mut volume_percent = 0.0;
    let mut is_muted = false;

    if let Some(vol_str) = volume_output.split_whitespace().nth(1) {
        if let Ok(vol_float) = vol_str.parse::<f64>() {
            volume_percent = vol_float * 100.0;
        }
    }

    if volume_output.contains("[MUTED]") {
        is_muted = true;
    }

    Ok(json!({
        "level": volume_percent.round(),
        "muted": is_muted,
        "source": "pipewire"
    }))
}

async fn get_media_status(connection: &Connection) -> Result<serde_json::Value> {
    let players = find_mpris_players(connection).await?;

    if players.is_empty() {
        return Ok(json!({
            "players": [],
            "active_player": null
        }));
    }

    let mut players_info = Vec::new();
    let mut active_player = None;

    for player in &players {
        if let Ok(player_info) = get_player_info(connection, player).await {
            if player_info.get("playback_status").and_then(|s| s.as_str()) == Some("Playing") {
                active_player = Some(player_info.clone());
            }
            players_info.push(player_info);
        }
    }

    Ok(json!({
        "players": players_info,
        "active_player": active_player
    }))
}

async fn find_mpris_players(connection: &Connection) -> Result<Vec<String>> {
    let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await?;
    let names = dbus_proxy.list_names().await?;

    let players = names
        .into_iter()
        .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
        .map(|name| name.to_string())
        .collect();

    Ok(players)
}

async fn get_player_info(connection: &Connection, player: &str) -> Result<serde_json::Value> {
    let player_proxy = zbus::Proxy::new(
        connection,
        player,
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )
    .await?;

    let identity_proxy = zbus::Proxy::new(
        connection,
        player,
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2",
    )
    .await?;

    let playback_status: String = player_proxy
        .get_property("PlaybackStatus")
        .await
        .unwrap_or_else(|_| "Unknown".to_string());

    let identity: String = identity_proxy
        .get_property("Identity")
        .await
        .unwrap_or_else(|_| player.to_string());

    let metadata: HashMap<String, zbus::zvariant::Value> = player_proxy
        .get_property("Metadata")
        .await
        .unwrap_or_default();

    let title = metadata
        .get("xesam:title")
        .and_then(|v| v.downcast_ref::<String>().ok())
        .unwrap_or("Unknown".to_owned());

    let artist = metadata
        .get("xesam:artist")
        .and_then(|v| v.clone().downcast::<Vec<String>>().ok())
        .and_then(|artists| artists.first().map(ToOwned::to_owned))
        .unwrap_or("Unknown".to_owned());

    Ok(json!({
        "player_name": identity,
        "playback_status": playback_status,
        "title": title,
        "artist": artist,
        "dbus_name": player
    }))
}
