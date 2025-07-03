use std::str::FromStr;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub completed_date: Option<DateTime<Utc>>,
    pub status: String,
    pub uid: String,
}

impl Task {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn is_completed(&self) -> bool {
        self.status == "COMPLETED"
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == "CANCELLED"
    }

    /// Fetch all tasks from Evolution Data Server with filtering options
    pub async fn all(
        include_completed: bool,
        include_cancelled: bool,
        due_within_days: u32,
    ) -> Result<Vec<Task>> {
        let connection = zbus::Connection::session().await?;
        let sources = crate::gnome::evolution::get_evolution_sources(&connection).await?;
        let mut all_tasks = Vec::new();

        for (_source_path, (info, _proxy)) in sources {
            if matches!(
                info.source_type,
                crate::gnome::evolution::SourceType::TaskList { .. }
            ) {
                let (task_list_path, bus_name) =
                    crate::gnome::evolution::open_task_list_source(&connection, &info.uid).await?;
                if let Ok(tasks) = Self::fetch_from_source(
                    &connection,
                    &task_list_path,
                    &bus_name,
                    include_completed,
                    include_cancelled,
                    due_within_days,
                )
                .await
                {
                    all_tasks.extend(tasks);
                }
            }
        }

        Ok(all_tasks)
    }

    /// Private helper to fetch tasks from a specific task list source
    async fn fetch_from_source(
        connection: &zbus::Connection,
        task_list_path: &str,
        bus_name: &str,
        include_completed: bool,
        include_cancelled: bool,
        due_within_days: u32,
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

        for ical_data in ical_objects {
            if let Ok(task) = Task::from_str(&ical_data) {
                // Apply filtering
                if !include_completed && task.is_completed() {
                    continue;
                }
                if !include_cancelled && task.is_cancelled() {
                    continue;
                }

                // Filter by due date if configured
                if due_within_days > 0 {
                    if let Some(due_date) = task.due_date {
                        let now = chrono::Utc::now();
                        let due_limit = now + chrono::Duration::days(due_within_days as i64);
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
}

impl FromStr for Task {
    type Err = anyhow::Error;

    fn from_str(ical_data: &str) -> Result<Self, Self::Err> {
        let ical = calcard::icalendar::ICalendar::parse(ical_data)
            .map_err(|e| anyhow::anyhow!("Failed to parse iCalendar data: {:?}", e))?;

        // Find the VTODO component within the VCALENDAR
        let todo_component = ical
            .components
            .iter()
            .find(|c| {
                matches!(
                    c.component_type,
                    calcard::icalendar::ICalendarComponentType::VTodo
                )
            })
            .ok_or_else(|| anyhow::anyhow!("No VTODO component found in iCalendar data"))?;

        let uid = todo_component
            .property(&calcard::icalendar::ICalendarProperty::Uid)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .unwrap_or_default();

        let summary = todo_component
            .property(&calcard::icalendar::ICalendarProperty::Summary)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let description = todo_component
            .property(&calcard::icalendar::ICalendarProperty::Description)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let due_date = todo_component
            .property(&calcard::icalendar::ICalendarProperty::Due)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let completed_date = todo_component
            .property(&calcard::icalendar::ICalendarProperty::Completed)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let status = todo_component
            .property(&calcard::icalendar::ICalendarProperty::Status)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .unwrap_or("NEEDS-ACTION")
            .to_string();

        Ok(Task {
            summary,
            description,
            due_date,
            completed_date,
            status,
            uid: uid.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};

    use super::*;

    #[test]
    fn test_task_from_str_complete() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VTODO
UID:test-task-123
SUMMARY:Test Task
DESCRIPTION:This is a test task
DUE:20240715T170000Z
STATUS:NEEDS-ACTION
END:VTODO
END:VCALENDAR"#;

        let task = Task::from_str(ical_data).unwrap();

        assert_eq!(task.uid, "test-task-123");
        assert_eq!(task.summary, Some("Test Task".to_string()));
        assert_eq!(task.description, Some("This is a test task".to_string()));
        assert!(task.due_date.is_some());
        assert_eq!(task.status, "NEEDS-ACTION");
        assert!(!task.is_completed());
        assert!(!task.is_cancelled());

        if let Some(due) = task.due_date {
            assert_eq!(due.year(), 2024);
            assert_eq!(due.month(), 7);
            assert_eq!(due.day(), 15);
            assert_eq!(due.hour(), 17);
            assert_eq!(due.minute(), 0);
        }
    }

    #[test]
    fn test_task_from_str_completed() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VTODO
UID:completed-task
SUMMARY:Completed Task
STATUS:COMPLETED
COMPLETED:20240710T120000Z
END:VTODO
END:VCALENDAR"#;

        let task = Task::from_str(ical_data).unwrap();

        assert_eq!(task.uid, "completed-task");
        assert_eq!(task.status, "COMPLETED");
        assert!(task.is_completed());
        assert!(!task.is_cancelled());
        assert!(task.completed_date.is_some());
    }

    #[test]
    fn test_task_from_str_cancelled() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VTODO
UID:cancelled-task
SUMMARY:Cancelled Task
STATUS:CANCELLED
END:VTODO
END:VCALENDAR"#;

        let task = Task::from_str(ical_data).unwrap();

        assert_eq!(task.uid, "cancelled-task");
        assert_eq!(task.status, "CANCELLED");
        assert!(!task.is_completed());
        assert!(task.is_cancelled());
    }

    #[test]
    fn test_task_from_str_default_status() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VTODO
UID:default-status-task
SUMMARY:Default Status Task
END:VTODO
END:VCALENDAR"#;

        let task = Task::from_str(ical_data).unwrap();

        assert_eq!(task.uid, "default-status-task");
        assert_eq!(task.status, "NEEDS-ACTION");
        assert!(!task.is_completed());
        assert!(!task.is_cancelled());
    }

    #[test]
    fn test_task_from_str_error_invalid_ical() {
        let ical_data = r#"INVALID DATA"#;

        let result = Task::from_str(ical_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_json_serialization() {
        let task = Task {
            uid: "test-123".to_string(),
            summary: Some("Test Summary".to_string()),
            description: None,
            due_date: None,
            completed_date: None,
            status: "NEEDS-ACTION".to_string(),
        };

        let json = task.to_json();
        assert!(json.is_object());
        assert_eq!(json["uid"], "test-123");
        assert_eq!(json["summary"], "Test Summary");
        assert_eq!(json["status"], "NEEDS-ACTION");
        assert!(json["description"].is_null());
    }

    #[test]
    fn test_task_with_various_date_formats() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VTODO
UID:various-dates-task
SUMMARY:Task with Various Date Formats
DUE;TZID=America/New_York:20240710T235959
COMPLETED:20240711T080000Z
STATUS:COMPLETED
END:VTODO
END:VCALENDAR"#;

        let task = Task::from_str(ical_data).unwrap();

        assert_eq!(task.uid, "various-dates-task");
        assert!(task.due_date.is_some());
        assert!(task.completed_date.is_some());
        assert!(task.is_completed());
    }

    // Unicode and special character tests
    #[test]
    fn test_task_with_special_characters_and_symbols() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VTODO
UID:special-chars-task
SUMMARY:Fix bug #12345 @urgent !!! & handle < > " ' characters
DESCRIPTION:Task involves: \n- HTML tags: <div class="test">\n- Quotes: "double" & 'single'\n- Symbols: @#$%^&*()+={}[]|\\:;?/>.<,~`
DUE:20240715T235959Z
PRIORITY:1
END:VTODO
END:VCALENDAR"#;

        let task = Task::from_str(ical_data).unwrap();

        assert_eq!(task.uid, "special-chars-task");
        assert_eq!(
            task.summary,
            Some("Fix bug #12345 @urgent !!! & handle < > \" ' characters".to_string())
        );
        assert!(task.description.as_ref().unwrap().contains("HTML tags"));
        assert!(task
            .description
            .as_ref()
            .unwrap()
            .contains("\"double\" & 'single'"));
        assert!(task.description.as_ref().unwrap().contains("@#$%^&*()"));
        // Note: categories are not currently stored in Task struct
    }
}
