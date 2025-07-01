use std::collections::HashMap;

use anyhow::Result;
use gio::glib;
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
