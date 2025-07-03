use anyhow::Result;
use serde_json::json;

use crate::{
    gnome::evolution::Event,
    mcp::{ResourceContent, ResourceProvider},
};

#[derive(Default)]
pub struct Calendar;

impl ResourceProvider for Calendar {
    const URI: &'static str = "gnome://calendar/events";
    const NAME: &'static str = "Calendar Events";
    const DESCRIPTION: &'static str = "Calendar events from Evolution Data Server";

    async fn get_content(&self) -> Result<ResourceContent> {
        let config = crate::config::CONFIG.get_calendar_config();
        let now = chrono::Utc::now();
        let start_time = now - chrono::Duration::days(config.days_behind as i64);
        let end_time = now + chrono::Duration::days(config.days_ahead as i64);

        let events = Event::all(start_time, end_time).await?;

        let events_json = json!({
            "events": events.iter().map(|e| e.to_json()).collect::<Vec<_>>(),
            "count": events.len()
        });

        Ok(ResourceContent {
            uri: Self::URI,
            mime_type: Self::MIME_TYPE,
            text: events_json.to_string(),
        })
    }
}
