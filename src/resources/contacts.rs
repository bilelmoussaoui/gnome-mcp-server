use std::str::FromStr;

use anyhow::Result;
use serde_json::json;

use crate::{
    gnome::evolution::{get_evolution_sources, open_address_book_source, Contact, SourceType},
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
            "contacts": contacts.iter().map(|c| c.to_json()).collect::<Vec<_>>(),
            "count": contacts.len()
        });

        Ok(ResourceContent {
            uri: Self::URI,
            mime_type: Self::MIME_TYPE,
            text: contacts_json.to_string(),
        })
    }
}

pub async fn get_contacts() -> Result<Vec<Contact>> {
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

    Ok(all_contacts)
}

async fn get_contact_objects(
    connection: &zbus::Connection,
    address_book_path: &str,
    bus_name: &str,
) -> Result<Vec<Contact>> {
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
        if let Ok(contact) = Contact::from_str(&contact_data) {
            // Apply configuration filtering
            let config = crate::config::CONFIG.get_contacts_config();
            if config.email_only && contact.emails.is_empty() {
                continue;
            }
            contacts.push(contact);
        }
    }

    proxy.call_method("Close", &()).await?;
    Ok(contacts)
}
