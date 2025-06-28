use crate::mcp::{Resource, ResourceContent};
use anyhow::Result;
use serde_json::json;
use zbus::Connection;

pub fn get_resource() -> Resource {
    Resource {
        uri: "gnome://system/info".to_owned(),
        name: "System Information".to_owned(),
        description: "OS version, hardware specs, uptime".to_owned(),
        mime_type: Some("application/json".to_owned()),
    }
}

pub async fn get_content() -> Result<ResourceContent> {
    let connection = Connection::system().await?;

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
        uri: "gnome://system/info".to_owned(),
        mime_type: "application/json".to_owned(),
        text: system_info.to_string(),
    })
}
