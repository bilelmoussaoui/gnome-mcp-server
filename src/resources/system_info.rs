use anyhow::Result;
use serde_json::json;

use crate::mcp::{ResourceContent, ResourceProvider};

#[derive(Default)]
pub struct SystemInfo;

impl ResourceProvider for SystemInfo {
    const URI: &'static str = "gnome://system/info";
    const NAME: &'static str = "System Information";
    const DESCRIPTION: &'static str = "OS version, hardware specs, uptime";

    async fn get_content(&self) -> Result<ResourceContent> {
        let connection = zbus::Connection::system().await?;

        let proxy = zbus::Proxy::new(
            &connection,
            "org.freedesktop.hostname1",
            "/org/freedesktop/hostname1",
            "org.freedesktop.hostname1",
        )
        .await?;

        let hostname: String = proxy.get_property("Hostname").await?;

        let proxy = zbus::Proxy::new(
            &connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await?;
        let boot_time: u64 = proxy
            .get_property("UserspaceTimestampMonotonic")
            .await
            .unwrap_or(0);

        let system_info = json!({
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "hostname": hostname,
            "boot_time": boot_time,
            "desktop_environment": "GNOME"
        });

        Ok(ResourceContent {
            uri: Self::URI,
            mime_type: Self::MIME_TYPE,
            text: system_info.to_string(),
        })
    }
}
