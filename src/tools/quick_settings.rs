use crate::mcp::{ToolParams, ToolProvider};
use crate::tool_params;
use anyhow::Result;
use gio::prelude::*;
use zbus::Connection;

#[derive(Default)]
pub struct QuickSettings;

tool_params! {
    QuickSettingsParams,
    required(setting: string, "Which boolean setting to toggle"),
    required(enabled: bool, "true to enable, false to disable the setting")
}

impl ToolProvider for QuickSettings {
    const NAME: &'static str = "quick_settings";
    const DESCRIPTION: &'static str =
        "Toggle boolean system settings (WiFi, Bluetooth, Night Light, etc.)";

    fn input_schema() -> serde_json::Value {
        QuickSettingsParams::input_schema()
    }

    async fn execute(&self, arguments: &serde_json::Value) -> Result<serde_json::Value> {
        let params = QuickSettingsParams::extract_params(arguments)?;

        Self::execute_with_result(|| execute_boolean_toggle(&params.setting, params.enabled)).await
    }
}

async fn execute_boolean_toggle(setting: &str, enabled: bool) -> Result<String> {
    match setting {
        "wifi" => toggle_wifi(enabled).await,
        "bluetooth" => toggle_bluetooth(enabled).await,
        "night_light" => toggle_night_light(enabled).await,
        "do_not_disturb" => toggle_do_not_disturb(enabled).await,
        "dark_style" => toggle_dark_style(enabled).await,
        _ => Err(anyhow::anyhow!("Unknown boolean setting: {}", setting)),
    }
}

async fn toggle_wifi(enabled: bool) -> Result<String> {
    let connection = Connection::system().await?;

    let nm_proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.NetworkManager",
    )
    .await?;

    nm_proxy.set_property("WirelessEnabled", enabled).await?;

    Ok(format!(
        "WiFi {}",
        if enabled { "enabled" } else { "disabled" }
    ))
}

async fn toggle_bluetooth(enabled: bool) -> Result<String> {
    let connection = Connection::system().await?;

    let adapter_proxy = zbus::Proxy::new(
        &connection,
        "org.bluez",
        "/org/bluez/hci0",
        "org.bluez.Adapter1",
    )
    .await?;

    adapter_proxy.set_property("Powered", enabled).await?;

    Ok(format!(
        "Bluetooth {}",
        if enabled { "enabled" } else { "disabled" }
    ))
}

async fn toggle_night_light(enabled: bool) -> Result<String> {
    let settings = gio::Settings::new("org.gnome.settings-daemon.plugins.color");
    settings.set_boolean("night-light-enabled", enabled)?;

    Ok(format!(
        "Night light {}",
        if enabled { "enabled" } else { "disabled" }
    ))
}

async fn toggle_dark_style(enabled: bool) -> Result<String> {
    let settings = gio::Settings::new("org.gnome.desktop.interface");
    let value = if enabled { "prefer-dark" } else { "default" };
    settings.set_string("color-scheme", value)?;

    Ok(format!(
        "Dark style {}",
        if enabled { "enabled" } else { "disabled" }
    ))
}

async fn toggle_do_not_disturb(enabled: bool) -> Result<String> {
    let settings = gio::Settings::new("org.gnome.desktop.notifications");
    // Note: show-banners logic is inverted (false = DND enabled)
    settings.set_boolean("show-banners", !enabled)?;

    Ok(format!(
        "Do Not Disturb {}",
        if enabled { "enabled" } else { "disabled" }
    ))
}
