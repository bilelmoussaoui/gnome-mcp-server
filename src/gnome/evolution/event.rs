use std::str::FromStr;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub uid: String,
    pub location: Option<String>,
    pub categories: Vec<String>,
    pub priority: Option<u32>,
    pub organizer: Option<String>,
    pub attendees: Vec<String>,
    pub status: Option<String>,
    pub transparency: Option<String>, // OPAQUE/TRANSPARENT
    pub class: Option<String>,        // PUBLIC/PRIVATE/CONFIDENTIAL
    pub created: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub url: Option<String>,
    pub rrule: Option<String>, // Recurrence rule
}

impl Event {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    /// Fetch all calendar events from Evolution Data Server within the
    /// specified time range
    pub async fn all(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Result<Vec<Event>> {
        let connection = zbus::Connection::session().await?;

        // Step 1: Get managed objects from SourceManager
        let sources = crate::gnome::evolution::get_evolution_sources(&connection).await?;

        // Step 2: Find calendar sources and get their events
        let mut all_events = Vec::new();

        for (_source_path, (info, _proxy)) in sources {
            if matches!(
                info.source_type,
                crate::gnome::evolution::SourceType::Calendar { .. }
            ) {
                let (calendar_path, bus_name) =
                    crate::gnome::evolution::open_calendar_source(&connection, &info.uid).await?;
                if let Ok(events) = Self::fetch_from_source(
                    &connection,
                    &calendar_path,
                    &bus_name,
                    start_time,
                    end_time,
                )
                .await
                {
                    all_events.extend(events);
                }
            }
        }

        Ok(all_events)
    }

    /// Private helper to fetch events from a specific calendar source
    async fn fetch_from_source(
        connection: &zbus::Connection,
        calendar_path: &str,
        bus_name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<Event>> {
        let mut events = Vec::new();

        let proxy = zbus::Proxy::new(
            connection,
            bus_name,
            calendar_path,
            "org.gnome.evolution.dataserver.Calendar",
        )
        .await?;

        let sexp_query = format!(
            "(occur-in-time-range? (make-time \\\"{}\\\") (make-time \\\"{}\\\"))",
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
}

impl FromStr for Event {
    type Err = anyhow::Error;

    fn from_str(ical_data: &str) -> Result<Self, Self::Err> {
        let ical = calcard::icalendar::ICalendar::parse(ical_data)
            .map_err(|e| anyhow::anyhow!("Failed to parse iCalendar data: {:?}", e))?;

        // Find the VEVENT component within the VCALENDAR
        let event_component = ical
            .components
            .iter()
            .find(|c| {
                matches!(
                    c.component_type,
                    calcard::icalendar::ICalendarComponentType::VEvent
                )
            })
            .ok_or_else(|| anyhow::anyhow!("No VEVENT component found in iCalendar data"))?;

        let uid = event_component
            .property(&calcard::icalendar::ICalendarProperty::Uid)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .unwrap_or_default();

        let summary = event_component
            .property(&calcard::icalendar::ICalendarProperty::Summary)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let description = event_component
            .property(&calcard::icalendar::ICalendarProperty::Description)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let start_time = event_component
            .property(&calcard::icalendar::ICalendarProperty::Dtstart)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let end_time = event_component
            .property(&calcard::icalendar::ICalendarProperty::Dtend)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let location = event_component
            .property(&calcard::icalendar::ICalendarProperty::Location)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let categories: Vec<String> = event_component
            .properties(&calcard::icalendar::ICalendarProperty::Categories)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let priority = event_component
            .property(&calcard::icalendar::ICalendarProperty::Priority)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_integer())
            .map(|i| i as u32);

        let organizer = event_component
            .property(&calcard::icalendar::ICalendarProperty::Organizer)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let attendees: Vec<String> = event_component
            .properties(&calcard::icalendar::ICalendarProperty::Attendee)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let status = event_component
            .property(&calcard::icalendar::ICalendarProperty::Status)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let transparency = event_component
            .property(&calcard::icalendar::ICalendarProperty::Transp)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let class = event_component
            .property(&calcard::icalendar::ICalendarProperty::Class)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let created = event_component
            .property(&calcard::icalendar::ICalendarProperty::Created)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let last_modified = event_component
            .property(&calcard::icalendar::ICalendarProperty::LastModified)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let url = event_component
            .property(&calcard::icalendar::ICalendarProperty::Url)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let rrule = event_component
            .property(&calcard::icalendar::ICalendarProperty::Rrule)
            .and_then(|p| p.values.first())
            .and_then(|v| match v {
                calcard::icalendar::ICalendarValue::RecurrenceRule(rule) => Some(rule.to_string()),
                _ => None,
            })
            .filter(|s| !s.is_empty());

        Ok(Event {
            summary,
            description,
            start_time,
            end_time,
            uid: uid.to_string(),
            location,
            categories,
            priority,
            organizer,
            attendees,
            status,
            transparency,
            class,
            created,
            last_modified,
            url,
            rrule,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_event_from_str_complete() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test-event-123
DTSTART:20240710T100000Z
DTEND:20240710T110000Z
SUMMARY:Test Event
DESCRIPTION:This is a test event
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "test-event-123");
        assert_eq!(event.summary, Some("Test Event".to_string()));
        assert_eq!(event.description, Some("This is a test event".to_string()));
        assert!(event.start_time.is_some());
        assert!(event.end_time.is_some());
    }

    #[test]
    fn test_event_from_str_minimal() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:minimal-event
DTSTART:20240710T100000Z
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "minimal-event");
        assert_eq!(event.summary, None);
        assert_eq!(event.description, None);
        assert!(event.start_time.is_some());
        assert_eq!(event.end_time, None);
    }

    #[test]
    fn test_event_from_str_empty_fields() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:empty-fields-event
DTSTART:20240710T100000Z
SUMMARY:
DESCRIPTION:
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "empty-fields-event");
        assert_eq!(event.summary, None);
        assert_eq!(event.description, None);
    }

    #[test]
    fn test_event_from_str_error_no_components() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
END:VCALENDAR"#;

        let result = Event::from_str(ical_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_event_from_str_error_invalid_ical() {
        let ical_data = r#"INVALID DATA"#;

        let result = Event::from_str(ical_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_event_json_serialization() {
        let event = Event {
            uid: "test-123".to_string(),
            summary: Some("Test Summary".to_string()),
            description: None,
            start_time: None,
            end_time: None,
            location: None,
            categories: vec![],
            priority: None,
            organizer: None,
            attendees: vec![],
            status: None,
            transparency: None,
            class: None,
            created: None,
            last_modified: None,
            url: None,
            rrule: None,
        };

        let json = event.to_json();
        assert!(json.is_object());
        assert_eq!(json["uid"], "test-123");
        assert_eq!(json["summary"], "Test Summary");
        assert!(json["description"].is_null());
    }

    // DateTime edge case tests
    #[test]
    fn test_event_with_different_timezone_formats() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:timezone-event
DTSTART;TZID=America/New_York:20240710T140000
DTEND;TZID=America/New_York:20240710T150000
SUMMARY:Timezone Test Event
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "timezone-event");
        assert_eq!(event.summary, Some("Timezone Test Event".to_string()));
        assert!(event.start_time.is_some());
        assert!(event.end_time.is_some());
    }

    #[test]
    fn test_event_with_date_only() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:date-only-event
DTSTART;VALUE=DATE:20240710
SUMMARY:All Day Event
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "date-only-event");
        assert_eq!(event.summary, Some("All Day Event".to_string()));

        if let Some(start) = event.start_time {
            assert_eq!(start.year(), 2024);
            assert_eq!(start.month(), 7);
            assert_eq!(start.day(), 10);
        }
    }

    #[test]
    fn test_event_with_malformed_dates() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:malformed-date-event
DTSTART:INVALID-DATE
DTEND:20240710T110000Z
SUMMARY:Event with bad start date
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "malformed-date-event");
        assert_eq!(event.summary, Some("Event with bad start date".to_string()));
        assert!(event.start_time.is_none());
        assert!(event.end_time.is_some());
    }

    // Unicode and special character tests
    #[test]
    fn test_event_with_unicode_characters() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:unicode-event-123
DTSTART:20240710T100000Z
DTEND:20240710T110000Z
SUMMARY:Meeting with José María Aznar in café 北京
DESCRIPTION:Discussing 中文 project with François and Müller. Price: €500.
LOCATION:Café 北京, Paris
CATEGORIES:Business,International
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "unicode-event-123");
        assert_eq!(
            event.summary,
            Some("Meeting with José María Aznar in café 北京".to_string())
        );
        assert_eq!(
            event.description,
            Some("Discussing 中文 project with François and Müller. Price: €500.".to_string())
        );

        // Test location field that now works
        assert_eq!(event.location, Some("Café 北京, Paris".to_string()));

        // Test categories field that now works
        assert_eq!(event.categories.len(), 2);
        assert!(event.categories.contains(&"Business".to_string()));
        assert!(event.categories.contains(&"International".to_string()));
    }

    #[test]
    fn test_whitespace_and_newline_handling() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:whitespace-event
DTSTART:20240710T100000Z
DTEND:20240710T110000Z
SUMMARY:  Event with lots of   spaces
DESCRIPTION:Multi-line\ndescription with\ttabs and\r\ncarriage returns.\n\nDouble newlines too.
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "whitespace-event");
        // The parser should preserve the whitespace as-is
        assert_eq!(
            event.summary,
            Some("  Event with lots of   spaces".to_string())
        );
        assert!(event.description.as_ref().unwrap().contains("Multi-line"));
        assert!(event.description.as_ref().unwrap().contains("tabs"));

        // Location field is now supported but not present in this test data
        assert_eq!(event.location, None);
    }

    #[test]
    fn test_escape_sequences_and_special_ical_chars() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:escaped-event
DTSTART:20240710T100000Z
DTEND:20240710T110000Z
SUMMARY:Event with "quotes" and \backslashes\
DESCRIPTION:Testing: \\backslash\, \;semicolon\, \,comma\, and \nnewline
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "escaped-event");
        assert!(event.summary.as_ref().unwrap().contains("\"quotes\""));
        assert!(event.summary.as_ref().unwrap().contains("backslashes"));
        assert!(event.description.as_ref().unwrap().contains("\\backslash"));
        assert!(event.description.as_ref().unwrap().contains(";semicolon"));

        // Location field is now supported but not present in this test data
        assert_eq!(event.location, None);
    }

    #[test]
    fn test_event_with_all_fields() {
        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:extended-event-123
DTSTART:20240710T100000Z
DTEND:20240710T110000Z
SUMMARY:Extended Event
DESCRIPTION:Event with location and categories
LOCATION:Conference Room A
CATEGORIES:Business,Meeting
PRIORITY:3
ORGANIZER:MAILTO:organizer@example.com
ATTENDEE:MAILTO:attendee1@example.com
ATTENDEE:MAILTO:attendee2@example.com
STATUS:CONFIRMED
TRANSP:OPAQUE
CLASS:PUBLIC
CREATED:20240701T080000Z
LAST-MODIFIED:20240705T120000Z
URL:https://example.com/event
RRULE:FREQ=WEEKLY;COUNT=4
END:VEVENT
END:VCALENDAR"#;

        let event = Event::from_str(ical_data).unwrap();

        assert_eq!(event.uid, "extended-event-123");
        assert_eq!(event.summary, Some("Extended Event".to_string()));
        assert_eq!(event.location, Some("Conference Room A".to_string()));
        assert_eq!(event.categories.len(), 2);
        assert!(event.categories.contains(&"Business".to_string()));
        assert!(event.categories.contains(&"Meeting".to_string()));

        // Test all the additional fields that should work
        assert_eq!(event.priority, Some(3));
        assert_eq!(
            event.organizer,
            Some("MAILTO:organizer@example.com".to_string())
        );
        assert_eq!(event.attendees.len(), 2);
        assert!(event
            .attendees
            .contains(&"MAILTO:attendee1@example.com".to_string()));
        assert!(event
            .attendees
            .contains(&"MAILTO:attendee2@example.com".to_string()));
        assert_eq!(event.status, Some("CONFIRMED".to_string()));
        assert_eq!(event.transparency, Some("OPAQUE".to_string()));
        assert_eq!(event.class, Some("PUBLIC".to_string()));
        assert!(event.created.is_some());
        assert!(event.last_modified.is_some());
        assert_eq!(event.url, Some("https://example.com/event".to_string()));
        assert_eq!(event.rrule, Some("FREQ=WEEKLY;COUNT=4".to_string()));
    }
}
