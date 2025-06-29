use anyhow::Result;
use serde_json::json;

use crate::{
    gnome::evolution::{get_evolution_sources, open_task_list_source, SourceType},
    mcp::{ResourceContent, ResourceProvider},
};

#[derive(Default)]
pub struct Tasks;

impl ResourceProvider for Tasks {
    const URI: &'static str = "gnome://tasks/list";
    const NAME: &'static str = "Task Lists";
    const DESCRIPTION: &'static str = "Task lists and todos from Evolution Data Server";

    async fn get_content(&self) -> Result<ResourceContent> {
        let tasks = get_task_lists().await?;

        let tasks_json = json!({
            "tasks": tasks,
            "count": tasks.len()
        });

        Ok(ResourceContent {
            uri: Self::URI,
            mime_type: Self::MIME_TYPE,
            text: tasks_json.to_string(),
        })
    }
}

pub async fn get_task_lists() -> Result<Vec<serde_json::Value>> {
    let connection = zbus::Connection::session().await?;
    let sources = get_evolution_sources(&connection).await?;
    let mut all_tasks = Vec::new();

    for (_source_path, (info, _proxy)) in sources {
        if matches!(info.source_type, SourceType::TaskList { .. }) {
            tracing::info!("Found task source {} named {}", info.uid, info.display_name);
            let (task_list_path, bus_name) = open_task_list_source(&connection, &info.uid).await?;
            tracing::info!(
                "Searching for tasks on path {} and bus name {}",
                task_list_path,
                bus_name
            );
            if let Ok(tasks) = get_task_objects(&connection, &task_list_path, &bus_name).await {
                all_tasks.extend(tasks);
            }
        }
    }

    if all_tasks.is_empty() {
        all_tasks.push(json!({
            "summary": "No Tasks Found",
            "description": "Connected to EDS but no tasks found",
            "created_time": chrono::Utc::now().to_rfc3339(),
            "source": "evolution-data-server",
            "status": "No tasks or task lists configured"
        }));
    }

    Ok(all_tasks)
}

async fn get_task_objects(
    connection: &zbus::Connection,
    task_list_path: &str,
    bus_name: &str,
) -> Result<Vec<serde_json::Value>> {
    let mut tasks = Vec::new();

    let proxy = zbus::Proxy::new(
        connection,
        bus_name,
        task_list_path,
        "org.gnome.evolution.dataserver.Calendar",
    )
    .await?;

    let response = proxy.call_method("GetObjectList", &("#t",)).await?;
    let ical_objects = response.body().deserialize::<Vec<String>>()?;

    tracing::info!("Found {} tasks", ical_objects.len());

    for ical_data in ical_objects {
        if let Some(task) = parse_ical_task(&ical_data) {
            tasks.push(task);
        }
    }
    Ok(tasks)
}

fn parse_ical_task(ical_data: &str) -> Option<serde_json::Value> {
    let config = crate::config::CONFIG.get_tasks_config();
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

    let due_date = component
        .property(&calcard::icalendar::ICalendarProperty::Due)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_partial_date_time())
        .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
        .map(|d| d.to_string())
        .unwrap_or_default();

    let completed = component
        .property(&calcard::icalendar::ICalendarProperty::Completed)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_partial_date_time())
        .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
        .map(|d| d.to_string());

    let status = component
        .property(&calcard::icalendar::ICalendarProperty::Status)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or("NEEDS-ACTION");

    let is_completed = completed.is_some();
    let is_cancelled = status == "CANCELLED";

    // Filter based on configuration
    if !config.include_completed && is_completed {
        return None;
    }
    if !config.include_cancelled && is_cancelled {
        return None;
    }

    // Filter by due date if configured
    if config.due_within_days > 0 {
        if let Some(due) = component
            .property(&calcard::icalendar::ICalendarProperty::Due)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
        {
            let now = chrono::Utc::now();
            let due_limit = now + chrono::Duration::days(config.due_within_days as i64);
            if due > due_limit {
                return None;
            }
        }
    }

    Some(json!({
        "summary": summary,
        "description": description,
        "due_date": due_date,
        "completed_date": completed,
        "status": status,
        "is_completed": is_completed,
        "uid": uid,
        "source": "evolution-tasks"
    }))
}
