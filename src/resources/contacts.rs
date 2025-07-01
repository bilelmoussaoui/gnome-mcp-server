use anyhow::Result;
use serde_json::json;

use crate::{
    gnome::evolution::{get_evolution_sources, open_address_book_source, SourceType},
    mcp::{ResourceContent, ResourceProvider},
};

#[derive(Default)]
pub struct Contacts;

impl ResourceProvider for Contacts {
    const URI: &'static str = "gnome://contacts/list";
    const NAME: &'static str = "Contacts";
    const DESCRIPTION: &'static str = "Contact list from Evolution Data Server";

    async fn get_content(&self) -> Result<ResourceContent> {
        let contacts = get_contacts().await?;

        let contacts_json = json!({
            "contacts": contacts,
            "count": contacts.len()
        });

        Ok(ResourceContent {
            uri: Self::URI,
            mime_type: Self::MIME_TYPE,
            text: contacts_json.to_string(),
        })
    }
}

pub async fn get_contacts() -> Result<Vec<serde_json::Value>> {
    let connection = zbus::Connection::session().await?;
    let sources = get_evolution_sources(&connection).await?;
    let mut all_contacts = Vec::new();

    for (_source_path, (info, _proxy)) in sources {
        if matches!(info.source_type, SourceType::AddressBook { .. }) {
            tracing::info!(
                "Found address book source {} named {}",
                info.uid,
                info.display_name
            );
            let (address_book_path, bus_name) =
                open_address_book_source(&connection, &info.uid).await?;
            tracing::info!(
                "Searching for contacts on path {} and bus name {}",
                address_book_path,
                bus_name
            );
            if let Ok(contacts) =
                get_contact_objects(&connection, &address_book_path, &bus_name).await
            {
                all_contacts.extend(contacts);
            }
        }
    }

    if all_contacts.is_empty() {
        all_contacts.push(json!({
            "name": "No Contacts Found",
            "description": "Connected to EDS but no contacts found",
            "source": "evolution-data-server",
            "status": "No contacts or address books configured"
        }));
    }

    Ok(all_contacts)
}

async fn get_contact_objects(
    connection: &zbus::Connection,
    address_book_path: &str,
    bus_name: &str,
) -> Result<Vec<serde_json::Value>> {
    let mut contacts = Vec::new();

    let proxy = zbus::Proxy::new(
        connection,
        bus_name,
        address_book_path,
        "org.gnome.evolution.dataserver.AddressBook",
    )
    .await?;

    proxy.call_method("Open", &()).await?;

    // Get all contacts using a simple query
    let response = proxy.call_method("GetContactList", &("",)).await.unwrap();
    let contact_strings = response.body().deserialize::<Vec<String>>()?;

    tracing::info!("Found {} contacts", contact_strings.len());

    for contact_data in contact_strings {
        if let Some(contact) = parse_vcard_contact(&contact_data) {
            contacts.push(contact);
        }
    }

    proxy.call_method("Close", &()).await?;
    Ok(contacts)
}

fn parse_vcard_contact(vcard_data: &str) -> Option<serde_json::Value> {
    let config = crate::config::CONFIG.get_contacts_config();
    let vcard = calcard::vcard::VCard::parse(vcard_data).ok()?;

    let full_name = vcard
        .property(&calcard::vcard::VCardProperty::Fn)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    let name = vcard
        .property(&calcard::vcard::VCardProperty::N)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    let nickname = vcard
        .property(&calcard::vcard::VCardProperty::Nickname)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

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
        .map(|d| d.to_string())
        .unwrap_or_default();

    let anniversary = vcard
        .property(&calcard::vcard::VCardProperty::Anniversary)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_partial_date_time())
        .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
        .map(|d| d.to_string())
        .unwrap_or_default();

    let organization = vcard
        .property(&calcard::vcard::VCardProperty::Org)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    let title = vcard
        .property(&calcard::vcard::VCardProperty::Title)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    let role = vcard
        .property(&calcard::vcard::VCardProperty::Role)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

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
        .unwrap_or_default();

    let language = vcard
        .property(&calcard::vcard::VCardProperty::Lang)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    let timezone = vcard
        .property(&calcard::vcard::VCardProperty::Tz)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    let geo = vcard
        .property(&calcard::vcard::VCardProperty::Geo)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    let revision = vcard
        .property(&calcard::vcard::VCardProperty::Rev)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_partial_date_time())
        .and_then(|d| d.to_date_time_with_tz(calcard::common::timezone::Tz::UTC))
        .map(|d| d.to_string())
        .unwrap_or_default();

    let key = vcard
        .property(&calcard::vcard::VCardProperty::Key)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    let pronouns = vcard
        .property(&calcard::vcard::VCardProperty::Pronouns)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

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
        .unwrap_or_default();

    let uid = vcard
        .property(&calcard::vcard::VCardProperty::Uid)
        .and_then(|p| p.values.first())
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    // Filter based on configuration
    if config.email_only && emails.is_empty() {
        return None;
    }

    Some(json!({
        "full_name": full_name,
        "name": name,
        "nickname": nickname,
        "emails": emails,
        "phones": phones,
        "impp": impp,
        "addresses": addresses,
        "birthday": birthday,
        "anniversary": anniversary,
        "organization": organization,
        "title": title,
        "role": role,
        "urls": urls,
        "categories": categories,
        "related": related,
        "gender": gender,
        "language": language,
        "timezone": timezone,
        "geo": geo,
        "revision": revision,
        "key": key,
        "pronouns": pronouns,
        "social_profiles": social_profiles,
        "note": note,
        "uid": uid,
        "source": "evolution-contacts"
    }))
}
