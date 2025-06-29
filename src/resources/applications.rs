use anyhow::Result;
use gio::prelude::*;
use serde_json::json;

use crate::mcp::{ResourceContent, ResourceProvider};

#[derive(Default)]
pub struct Applications;

impl ResourceProvider for Applications {
    const URI: &'static str = "gnome://applications/installed";
    const NAME: &'static str = "Installed Applications";
    const DESCRIPTION: &'static str = "List of installed desktop applications";

    async fn get_content(&self) -> Result<ResourceContent> {
        // Use GIO to get all applications properly
        let mut applications = Vec::new();

        // Get all installed applications via GIO
        let app_infos = gio::AppInfo::all();

        for app_info in app_infos {
            // Skip apps that shouldn't be displayed
            if !app_info.should_show() {
                continue;
            }

            let name = app_info.name().to_string();
            let description = app_info
                .description()
                .map(|d| d.to_string())
                .unwrap_or_default();
            let executable = app_info.executable().to_string_lossy().to_string();
            let id = app_info.id().map(|i| i.to_string()).unwrap_or_default();

            applications.push(json!({
                "name": name,
                "description": description,
                "executable": executable,
                "id": id,
                "can_launch": true
            }));
        }

        // Sort by name
        applications.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        });

        let apps_json = json!({
            "applications": applications,
            "count": applications.len()
        });

        Ok(ResourceContent {
            uri: Self::URI,
            mime_type: Self::MIME_TYPE,
            text: apps_json.to_string(),
        })
    }
}
