use anyhow::Result;
use serde_json::json;

use crate::{
    gnome::evolution::Contact,
    mcp::{ResourceContent, ResourceProvider},
};

#[derive(Default)]
pub struct Contacts;

impl ResourceProvider for Contacts {
    const URI: &'static str = "gnome://contacts/list";
    const NAME: &'static str = "Contacts";
    const DESCRIPTION: &'static str = "Contact list from Evolution Data Server";

    async fn get_content(&self) -> Result<ResourceContent> {
        let config = crate::config::CONFIG.get_contacts_config();
        let contacts = Contact::all(config.email_only).await?;

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
