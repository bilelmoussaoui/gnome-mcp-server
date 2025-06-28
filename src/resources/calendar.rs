use crate::{
    gnome::evolution::SourceType,
    mcp::{Resource, ResourceContent},
};
use anyhow::Result;
use serde_json::json;
use zbus::Connection;

pub fn get_resource() -> Resource {
    Resource {
        uri: "gnome://calendar/events".to_owned(),
        name: "Calendar Events".to_owned(),
        description: "Calendar events from Evolution Data Server".to_owned(),
        mime_type: Some("application/json".to_owned()),
    }
}

pub async fn get_content() -> Result<ResourceContent> {
    let events = get_calendar_events().await?;

    let events_json = json!({
        "events": events,
        "count": events.len()
    });

    Ok(ResourceContent {
        uri: "gnome://calendar/events".to_owned(),
        mime_type: "application/json".to_owned(),
        text: events_json.to_string(),
    })
}

pub async fn get_calendar_events() -> Result<Vec<serde_json::Value>> {
    let connection = Connection::session().await?;

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

    if all_events.is_empty() {
        all_events.push(json!({
            "summary": "Evolution Data Server",
            "description": "Connected to EDS but no calendar events found",
            "start_time": chrono::Utc::now().to_rfc3339(),
            "source": "evolution-data-server",
            "status": "No events or calendars configured"
        }));
    }

    Ok(all_events)
}

async fn get_calendar_objects(
    connection: &Connection,
    calendar_path: &str,
    bus_name: &str,
) -> Result<Vec<serde_json::Value>> {
    let mut events = Vec::new();

    let proxy = zbus::Proxy::new(
        connection,
        bus_name,
        calendar_path,
        "org.gnome.evolution.dataserver.Calendar",
    )
    .await?;

    // Get upcoming events (next 30 days)
    let now = chrono::Utc::now();
    let month_later = now + chrono::Duration::days(30);

    let sexp_query = format!(
        "(occur-in-time-range? (make-time \"{}\") (make-time \"{}\"))",
        now.format("%Y%m%dT%H%M%SZ"),
        month_later.format("%Y%m%dT%H%M%SZ")
    );

    let response = proxy.call_method("GetObjectList", &(sexp_query,)).await?;
    let ical_objects = response.body().deserialize::<Vec<String>>()?;

    for ical_data in ical_objects {
        if let Some(event) = parse_ical_event(&ical_data) {
            events.push(event);
        }
    }

    Ok(events)
}

fn parse_ical_event(ical_data: &str) -> Option<serde_json::Value> {
    let ical = calcard::icalendar::ICalendar::parse(ical_data).ok()?;
    let component = ical.components.first()?;
    let uid = component
        .property(&calcard::icalendar::ICalendarProperty::Uid)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();
    let summary = component
        .property(&calcard::icalendar::ICalendarProperty::Summary)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();
    let description = component
        .property(&calcard::icalendar::ICalendarProperty::Description)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();
    let start_time = component
        .property(&calcard::icalendar::ICalendarProperty::Dtstart)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_partial_date_time())
        .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
        .map(|d| d.to_string())
        .unwrap_or_default();
    let end_time = component
        .property(&calcard::icalendar::ICalendarProperty::Dtend)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_partial_date_time())
        .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
        .map(|d| d.to_string())
        .unwrap_or_default();

    Some(json!({
        "summary": summary,
        "description": description,
        "start_time": start_time,
        "end_time": end_time,
        "uid": uid,
        "source": "evolution-calendar"
    }))
}
