use anyhow::Result;
use serde_json::json;

use crate::{
    gnome::evolution::Task,
    mcp::{ResourceContent, ResourceProvider},
};

#[derive(Default)]
pub struct Tasks;

impl ResourceProvider for Tasks {
    const URI: &'static str = "gnome://tasks/list";
    const NAME: &'static str = "Task Lists";
    const DESCRIPTION: &'static str = "Task lists and todos from Evolution Data Server";

    async fn get_content(&self) -> Result<ResourceContent> {
        let config = crate::config::CONFIG.get_tasks_config();
        let tasks = Task::all(
            config.include_completed,
            config.include_cancelled,
            config.due_within_days,
        )
        .await?;

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
