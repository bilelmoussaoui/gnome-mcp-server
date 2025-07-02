use std::str::FromStr;

use anyhow::Result;
use serde_json::json;

use crate::{
    gnome::evolution::{get_evolution_sources, open_task_list_source, SourceType, Task},
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
            "tasks": tasks.iter().map(|t| t.to_json()).collect::<Vec<_>>(),
            "count": tasks.len()
        });

        Ok(ResourceContent {
            uri: Self::URI,
            mime_type: Self::MIME_TYPE,
            text: tasks_json.to_string(),
        })
    }
}

pub async fn get_task_lists() -> Result<Vec<Task>> {
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

    Ok(all_tasks)
}

async fn get_task_objects(
    connection: &zbus::Connection,
    task_list_path: &str,
    bus_name: &str,
) -> Result<Vec<Task>> {
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
        if let Ok(task) = Task::from_str(&ical_data) {
            // Apply configuration filtering
            let config = crate::config::CONFIG.get_tasks_config();

            // Filter based on configuration
            if !config.include_completed && task.is_completed() {
                continue;
            }
            if !config.include_cancelled && task.is_cancelled() {
                continue;
            }

            // Filter by due date if configured
            if config.due_within_days > 0 {
                if let Some(due_date) = task.due_date {
                    let now = chrono::Utc::now();
                    let due_limit = now + chrono::Duration::days(config.due_within_days as i64);
                    if due_date > due_limit {
                        continue;
                    }
                }
            }

            tasks.push(task);
        }
    }
    Ok(tasks)
}
