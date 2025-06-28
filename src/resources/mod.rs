pub mod applications;
pub mod audio;
pub mod calendar;
pub mod system_info;
pub mod tasks;

use crate::mcp::{Resource, ResourceContent};

pub fn list_resources() -> Vec<Resource> {
    vec![
        system_info::get_resource(),
        applications::get_resource(),
        calendar::get_resource(),
        tasks::get_resource(),
        audio::get_resource(),
    ]
}

pub async fn resource_for_uri(uri: &str) -> anyhow::Result<ResourceContent> {
    match uri {
        "gnome://system/info" => system_info::get_content().await,
        "gnome://applications/installed" => applications::get_content().await,
        "gnome://calendar/events" => calendar::get_content().await,
        "gnome://tasks/list" => tasks::get_content().await,
        "gnome://audio/status" => audio::get_content().await,
        _ => anyhow::bail!("Unsupported URI {uri}"),
    }
}
