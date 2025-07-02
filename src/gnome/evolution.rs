use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use chrono::{DateTime, Utc};
use gio::glib;
use serde::{Deserialize, Serialize};
use zbus::zvariant::OwnedObjectPath;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SourceInfo {
    pub uid: String,
    pub path: OwnedObjectPath,
    pub display_name: String,
    pub enabled: bool,
    pub source_type: SourceType,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SourceType {
    Calendar { backend_name: String },
    TaskList { backend_name: String },
    AddressBook { backend_name: String },
}

fn parse_source_data(path: OwnedObjectPath, uid: String, data: String) -> Option<SourceInfo> {
    let key_file = glib::KeyFile::new();
    key_file
        .load_from_data(&data, glib::KeyFileFlags::NONE)
        .ok()?;

    // Check if source is enabled
    let enabled = key_file.boolean("Data Source", "Enabled").unwrap_or(false);
    if !enabled {
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

pub async fn get_evolution_sources(
    connection: &zbus::Connection,
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

        if let Some(source_info) = parse_source_data(object_path.clone(), uid, data) {
            sources.insert(object_path, (source_info, proxy));
        }
    }
    Ok(sources)
}

pub async fn open_calendar_source(
    connection: &zbus::Connection,
    source_uid: &str,
) -> Result<(String, String)> {
    let proxy = zbus::Proxy::new(
        connection,
        "org.gnome.evolution.dataserver.Calendar8",
        "/org/gnome/evolution/dataserver/CalendarFactory",
        "org.gnome.evolution.dataserver.CalendarFactory",
    )
    .await?;

    let response = proxy.call_method("OpenCalendar", &(source_uid,)).await?;
    let (calendar_path, bus_name) = response.body().deserialize::<(String, String)>()?;
    Ok((calendar_path, bus_name))
}

pub async fn open_task_list_source(
    connection: &zbus::Connection,
    source_uid: &str,
) -> Result<(String, String)> {
    let proxy = zbus::Proxy::new(
        connection,
        "org.gnome.evolution.dataserver.Calendar8",
        "/org/gnome/evolution/dataserver/CalendarFactory",
        "org.gnome.evolution.dataserver.CalendarFactory",
    )
    .await?;

    let response = proxy.call_method("OpenTaskList", &(source_uid,)).await?;
    let (task_list_path, bus_name) = response.body().deserialize::<(String, String)>()?;
    Ok((task_list_path, bus_name))
}

pub async fn open_address_book_source(
    connection: &zbus::Connection,
    source_uid: &str,
) -> Result<(String, String)> {
    let proxy = zbus::Proxy::new(
        connection,
        "org.gnome.evolution.dataserver.AddressBook10",
        "/org/gnome/evolution/dataserver/AddressBookFactory",
        "org.gnome.evolution.dataserver.AddressBookFactory",
    )
    .await?;

    let response = proxy.call_method("OpenAddressBook", &(source_uid,)).await?;
    let (address_book_path, bus_name) = response.body().deserialize::<(String, String)>()?;
    Ok((address_book_path, bus_name))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub uid: String,
}

impl Event {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

impl FromStr for Event {
    type Err = anyhow::Error;

    fn from_str(ical_data: &str) -> Result<Self, Self::Err> {
        let ical = calcard::icalendar::ICalendar::parse(ical_data)
            .map_err(|e| anyhow::anyhow!("Failed to parse iCalendar data: {:?}", e))?;
        let component = ical
            .components
            .first()
            .ok_or_else(|| anyhow::anyhow!("No components found in iCalendar data"))?;

        let uid = component
            .property(&calcard::icalendar::ICalendarProperty::Uid)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .unwrap_or_default();

        let summary = component
            .property(&calcard::icalendar::ICalendarProperty::Summary)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let description = component
            .property(&calcard::icalendar::ICalendarProperty::Description)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let start_time = component
            .property(&calcard::icalendar::ICalendarProperty::Dtstart)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let end_time = component
            .property(&calcard::icalendar::ICalendarProperty::Dtend)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        Ok(Event {
            summary,
            description,
            start_time,
            end_time,
            uid: uid.to_string(),
        })
    }
}

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
        self.completed_date.is_some()
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == "CANCELLED"
    }
}

impl FromStr for Task {
    type Err = anyhow::Error;

    fn from_str(ical_data: &str) -> Result<Self, Self::Err> {
        let ical = calcard::icalendar::ICalendar::parse(ical_data)
            .map_err(|e| anyhow::anyhow!("Failed to parse iCalendar data: {:?}", e))?;
        let component = ical
            .components
            .first()
            .ok_or_else(|| anyhow::anyhow!("No components found in iCalendar data"))?;

        let uid = component
            .property(&calcard::icalendar::ICalendarProperty::Uid)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .unwrap_or_default();

        let summary = component
            .property(&calcard::icalendar::ICalendarProperty::Summary)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let description = component
            .property(&calcard::icalendar::ICalendarProperty::Description)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let due_date = component
            .property(&calcard::icalendar::ICalendarProperty::Due)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let completed_date = component
            .property(&calcard::icalendar::ICalendarProperty::Completed)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let status = component
            .property(&calcard::icalendar::ICalendarProperty::Status)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .unwrap_or("NEEDS-ACTION");

        Ok(Task {
            summary,
            description,
            due_date,
            completed_date,
            status: status.to_string(),
            uid: uid.to_string(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub full_name: Option<String>,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub emails: Vec<String>,
    pub phones: Vec<String>,
    pub impp: Vec<String>,
    pub addresses: Vec<String>,
    pub birthday: Option<DateTime<Utc>>,
    pub anniversary: Option<DateTime<Utc>>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub role: Option<String>,
    pub urls: Vec<String>,
    pub categories: Vec<String>,
    pub related: Vec<String>,
    pub gender: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub geo: Option<String>,
    pub revision: Option<DateTime<Utc>>,
    pub key: Option<String>,
    pub pronouns: Option<String>,
    pub social_profiles: Vec<String>,
    pub note: Option<String>,
    pub uid: String,
}

impl Contact {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

impl FromStr for Contact {
    type Err = anyhow::Error;

    fn from_str(vcard_data: &str) -> Result<Self, Self::Err> {
        let vcard = calcard::vcard::VCard::parse(vcard_data)
            .map_err(|e| anyhow::anyhow!("Failed to parse vCard data: {:?}", e))?;

        let full_name = vcard
            .property(&calcard::vcard::VCardProperty::Fn)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let name = vcard
            .property(&calcard::vcard::VCardProperty::N)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let nickname = vcard
            .property(&calcard::vcard::VCardProperty::Nickname)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let emails: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Email)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .map(|s| s.to_string())
            .collect();

        let phones: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Tel)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .map(|s| s.to_string())
            .collect();

        let impp: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Impp)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .map(|s| s.to_string())
            .collect();

        let addresses: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Adr)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .map(|s| s.to_string())
            .collect();

        let birthday = vcard
            .property(&calcard::vcard::VCardProperty::Bday)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let anniversary = vcard
            .property(&calcard::vcard::VCardProperty::Anniversary)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let organization = vcard
            .property(&calcard::vcard::VCardProperty::Org)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let title = vcard
            .property(&calcard::vcard::VCardProperty::Title)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let role = vcard
            .property(&calcard::vcard::VCardProperty::Role)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let urls: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Url)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .map(|s| s.to_string())
            .collect();

        let categories: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Categories)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .map(|s| s.to_string())
            .collect();

        let related: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Related)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .map(|s| s.to_string())
            .collect();

        let gender = vcard
            .property(&calcard::vcard::VCardProperty::Gender)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let language = vcard
            .property(&calcard::vcard::VCardProperty::Lang)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let timezone = vcard
            .property(&calcard::vcard::VCardProperty::Tz)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let geo = vcard
            .property(&calcard::vcard::VCardProperty::Geo)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let revision = vcard
            .property(&calcard::vcard::VCardProperty::Rev)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_partial_date_time())
            .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
            .map(|dt| dt.with_timezone(&Utc));

        let key = vcard
            .property(&calcard::vcard::VCardProperty::Key)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let pronouns = vcard
            .property(&calcard::vcard::VCardProperty::Pronouns)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let social_profiles: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Socialprofile)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .map(|s| s.to_string())
            .collect();

        let note = vcard
            .property(&calcard::vcard::VCardProperty::Note)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let uid = vcard
            .property(&calcard::vcard::VCardProperty::Uid)
            .and_then(|p| p.values.first())
            .and_then(|v| v.as_text())
            .unwrap_or_default();

        Ok(Contact {
            full_name,
            name,
            nickname,
            emails,
            phones,
            impp,
            addresses,
            birthday,
            anniversary,
            organization,
            title,
            role,
            urls,
            categories,
            related,
            gender,
            language,
            timezone,
            geo,
            revision,
            key,
            pronouns,
            social_profiles,
            note,
            uid: uid.to_string(),
        })
    }
}
