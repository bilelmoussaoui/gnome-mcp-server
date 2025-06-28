use crate::mcp::{Resource, ResourceContent};
use anyhow::Result;
use gio::glib;
use serde_json::json;
use std::collections::HashMap;
use zbus::{Connection, zvariant::OwnedObjectPath};

#[derive(Debug)]
#[allow(dead_code)]
struct SourceInfo {
    uid: String,
    path: OwnedObjectPath,
    display_name: String,
    enabled: bool,
    source_type: SourceType,
}

#[derive(Debug)]
#[allow(dead_code)]
enum SourceType {
    Calendar { backend_name: String },
    TaskList { backend_name: String },
    AddressBook { backend_name: String },
}

fn parse_source_data(path: OwnedObjectPath, uid: String, data: &str) -> Option<SourceInfo> {
    let key_file = glib::KeyFile::new();
    key_file
        .load_from_data(data, glib::KeyFileFlags::NONE)
        .unwrap();

    // Check if source is enabled
    let enabled = key_file.boolean("Data Source", "Enabled").unwrap_or(false);
    if !enabled {
        println!("Source is disabled");
        return None;
    }

    let display_name = key_file
        .string("Data Source", "DisplayName")
        .unwrap_or_else(|_| "Unknown".into());

    // Check what type of source this is
    let source_type = if key_file.has_group("Calendar") {
        let backend_name = key_file
            .string("Calendar", "BackendName")
            .unwrap_or_else(|_| "unknown".into());
        SourceType::Calendar {
            backend_name: backend_name.to_string(),
        }
    } else if key_file.has_group("Task List") {
        let backend_name = key_file
            .string("Task List", "BackendName")
            .unwrap_or_else(|_| "unknown".into());
        SourceType::TaskList {
            backend_name: backend_name.to_string(),
        }
    } else if key_file.has_group("Address Book") {
        let backend_name = key_file
            .string("Address Book", "BackendName")
            .unwrap_or_else(|_| "unknown".into());
        SourceType::AddressBook {
            backend_name: backend_name.to_string(),
        }
    } else {
        println!("Unknown source type");
        return None;
    };

    Some(SourceInfo {
        uid,
        path,
        display_name: display_name.to_string(),
        enabled,
        source_type,
    })
}

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
    let sources = get_evolution_sources(&connection).await?;

    // Step 2: Find calendar sources and get their events
    let mut all_events = Vec::new();

    for (_source_path, (info, _proxy)) in sources {
        if matches!(info.source_type, SourceType::Calendar { .. }) {
            if let Ok(events) = get_calender_events_from_source(&connection, &info).await {
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

async fn get_evolution_sources(
    connection: &Connection,
) -> Result<HashMap<OwnedObjectPath, (SourceInfo, zbus::Proxy<'static>)>> {
    let proxy = zbus::fdo::ObjectManagerProxy::builder(connection)
        .destination("org.gnome.evolution.dataserver.Sources5")?
        .path("/org/gnome/evolution/dataserver/SourceManager")?
        .build()
        .await?;
    let mut sources = HashMap::new();

    // Get all managed objects
    let objects = proxy.get_managed_objects().await?;
    for (object_path, _) in objects {
        let proxy = zbus::Proxy::new(
            connection,
            "org.gnome.evolution.dataserver.Sources5",
            object_path.clone(),
            "org.gnome.evolution.dataserver.Source",
        )
        .await?;
        let data = proxy.get_property::<String>("Data").await?;
        let uid = proxy.get_property::<String>("UID").await?;

        if let Some(source_info) = parse_source_data(object_path.clone(), uid, &data) {
            sources.insert(object_path, (source_info, proxy));
        }
    }
    Ok(sources)
}

async fn get_calender_events_from_source(
    connection: &Connection,
    info: &SourceInfo,
) -> anyhow::Result<Vec<serde_json::Value>> {
    // Try to get the calendar factory to open this source
    let proxy = zbus::Proxy::new(
        connection,
        "org.gnome.evolution.dataserver.Calendar8",
        "/org/gnome/evolution/dataserver/CalendarFactory",
        "org.gnome.evolution.dataserver.CalendarFactory",
    )
    .await?;
    // Try to open the calendar with this source UID
    let response = proxy.call_method("OpenCalendar", &(&info.uid,)).await?;
    let (calendar_path, bus_name) = response.body().deserialize::<(String, String)>()?;
    get_calendar_objects(connection, &calendar_path, &bus_name).await
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
