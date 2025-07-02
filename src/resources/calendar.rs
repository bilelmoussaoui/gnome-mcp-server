use std::str::FromStr;

use anyhow::Result;
use serde_json::json;

use crate::{
    gnome::evolution::{Event, SourceType},
    mcp::{ResourceContent, ResourceProvider},
};

#[derive(Default)]
pub struct Calendar;

impl ResourceProvider for Calendar {
    const URI: &'static str = "gnome://calendar/events";
    const NAME: &'static str = "Calendar Events";
    const DESCRIPTION: &'static str = "Calendar events from Evolution Data Server";

    async fn get_content(&self) -> Result<ResourceContent> {
        let events = get_calendar_events().await?;

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

pub async fn get_calendar_events() -> Result<Vec<Event>> {
    let connection = zbus::Connection::session().await?;

    // Step 1: Get managed objects from SourceManager
    let sources = crate::gnome::evolution::get_evolution_sources(&connection).await?;

    // Step 2: Find calendar sources and get their events
    let mut all_events = Vec::new();

    for (_source_path, (info, _proxy)) in sources {
        if matches!(info.source_type, SourceType::Calendar { .. }) {
            let (calendar_path, bus_name) =
                crate::gnome::evolution::open_calendar_source(&connection, &info.uid).await?;
            if let Ok(events) = get_calendar_objects(&connection, &calendar_path, &bus_name).await {
                all_events.extend(events);
            }
        }
    }

    Ok(all_events)
}

async fn get_calendar_objects(
    connection: &zbus::Connection,
    calendar_path: &str,
    bus_name: &str,
) -> Result<Vec<Event>> {
    let mut events = Vec::new();

    let proxy = zbus::Proxy::new(
        connection,
        bus_name,
        calendar_path,
        "org.gnome.evolution.dataserver.Calendar",
    )
    .await?;

    // Get events based on configuration
    let config = crate::config::CONFIG.get_calendar_config();
    let now = chrono::Utc::now();
    let start_time = now - chrono::Duration::days(config.days_behind as i64);
    let end_time = now + chrono::Duration::days(config.days_ahead as i64);

    let sexp_query = format!(
        "(occur-in-time-range? (make-time \"{}\") (make-time \"{}\"))",
        start_time.format("%Y%m%dT%H%M%SZ"),
        end_time.format("%Y%m%dT%H%M%SZ")
    );

    let response = proxy.call_method("GetObjectList", &(sexp_query,)).await?;
    let ical_objects = response.body().deserialize::<Vec<String>>()?;

    for ical_data in ical_objects {
        if let Ok(event) = Event::from_str(&ical_data) {
            events.push(event);
        }
    }

    Ok(events)
}
