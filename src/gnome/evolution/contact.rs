use std::str::FromStr;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub note: Option<String>,
    pub uid: String,
}

impl Contact {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    /// Fetch all contacts from Evolution Data Server with filtering options
    pub async fn all(email_only: bool) -> Result<Vec<Contact>> {
        let connection = zbus::Connection::session().await?;
        let sources = crate::gnome::evolution::get_evolution_sources(&connection).await?;
        let mut all_contacts = Vec::new();

        for (_source_path, (info, _proxy)) in sources {
            if matches!(
                info.source_type,
                crate::gnome::evolution::SourceType::AddressBook { .. }
            ) {
                let (address_book_path, bus_name) =
                    crate::gnome::evolution::open_address_book_source(&connection, &info.uid)
                        .await?;
                if let Ok(contacts) =
                    Self::fetch_from_source(&connection, &address_book_path, &bus_name, email_only)
                        .await
                {
                    all_contacts.extend(contacts);
                }
            }
        }

        Ok(all_contacts)
    }

    /// Private helper to fetch contacts from a specific address book source
    async fn fetch_from_source(
        connection: &zbus::Connection,
        address_book_path: &str,
        bus_name: &str,
        email_only: bool,
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
        let response = proxy.call_method("GetContactList", &("",)).await?;
        let contact_strings = response.body().deserialize::<Vec<String>>()?;

        for contact_data in contact_strings {
            if let Ok(contact) = Contact::from_str(&contact_data) {
                // Apply configuration filtering
                if email_only && contact.emails.is_empty() {
                    continue;
                }
                contacts.push(contact);
            }
        }

        proxy.call_method("Close", &()).await?;
        Ok(contacts)
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
            .map(|p| {
                // N field has multiple components: Last;First;Middle;Prefix;Suffix
                p.values
                    .iter()
                    .filter_map(|v| v.as_text())
                    .collect::<Vec<_>>()
                    .join(";")
            })
            .filter(|s| !s.is_empty());

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
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let phones: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Tel)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let impp: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Impp)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let addresses: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Adr)
            .map(|p| {
                // ADR has multiple components, join them with semicolons
                p.values
                    .iter()
                    .filter_map(|v| v.as_text())
                    .collect::<Vec<_>>()
                    .join(";")
            })
            .filter(|s| !s.is_empty())
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
            .map(|p| {
                // ORG field has multiple components: Organization;Department1;Department2;...
                p.values
                    .iter()
                    .filter_map(|v| v.as_text())
                    .collect::<Vec<_>>()
                    .join(";")
            })
            .filter(|s| !s.is_empty());

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
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let categories: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Categories)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let related: Vec<String> = vcard
            .properties(&calcard::vcard::VCardProperty::Related)
            .flat_map(|p| &p.values)
            .filter_map(|v| v.as_text())
            .filter(|s| !s.is_empty())
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
            note,
            uid: uid.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_contact_from_str_complete() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:test-contact-123
FN:John Doe
N:Doe;John;;;
EMAIL:john@example.com
TEL:+1-555-123-4567
ORG:Test Company
TITLE:Software Engineer
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "test-contact-123");
        assert_eq!(contact.full_name, Some("John Doe".to_string()));
        assert_eq!(contact.name, Some("Doe;John;;;".to_string()));
        assert_eq!(contact.emails, vec!["john@example.com"]);
        assert_eq!(contact.phones, vec!["+1-555-123-4567"]);
        assert_eq!(contact.organization, Some("Test Company".to_string()));
        assert_eq!(contact.title, Some("Software Engineer".to_string()));
    }

    #[test]
    fn test_contact_from_str_minimal() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:minimal-contact
FN:Jane Smith
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "minimal-contact");
        assert_eq!(contact.full_name, Some("Jane Smith".to_string()));
        assert_eq!(contact.name, None);
        assert!(contact.emails.is_empty());
        assert!(contact.phones.is_empty());
    }

    #[test]
    fn test_contact_from_str_empty_fields() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:empty-fields-contact
FN:
N:
EMAIL:
TEL:
ORG:
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "empty-fields-contact");
        assert_eq!(contact.full_name, None);
        assert_eq!(contact.name, None);
        assert!(contact.emails.is_empty());
        assert!(contact.phones.is_empty());
        assert_eq!(contact.organization, None);
    }

    #[test]
    fn test_contact_from_str_error_invalid_vcard() {
        let vcard_data = r#"INVALID DATA"#;

        let result = Contact::from_str(vcard_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_contact_json_serialization() {
        let contact = Contact {
            uid: "test-123".to_string(),
            full_name: Some("Test Person".to_string()),
            name: None,
            nickname: None,
            emails: vec!["test@example.com".to_string()],
            phones: vec![],
            impp: vec![],
            addresses: vec![],
            birthday: None,
            anniversary: None,
            organization: None,
            title: None,
            role: None,
            urls: vec![],
            categories: vec![],
            related: vec![],
            gender: None,
            language: None,
            timezone: None,
            geo: None,
            note: None,
        };

        let json = contact.to_json();
        assert!(json.is_object());
        assert_eq!(json["uid"], "test-123");
        assert_eq!(json["full_name"], "Test Person");
        assert_eq!(json["emails"], serde_json::json!(["test@example.com"]));
        assert!(json["name"].is_null());
    }

    // Multi-value field tests
    #[test]
    fn test_contact_with_multiple_emails_and_phones() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:multi-email-phone-contact
FN:Multi Contact Person
EMAIL;TYPE=WORK:work@example.com
EMAIL;TYPE=HOME:home@example.com
EMAIL:other@example.com
TEL;TYPE=WORK:+1-555-123-4567
TEL;TYPE=HOME:+1-555-987-6543
TEL;TYPE=CELL:+1-555-555-5555
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "multi-email-phone-contact");

        // Should capture all emails
        assert_eq!(contact.emails.len(), 3);
        assert!(contact.emails.contains(&"work@example.com".to_string()));
        assert!(contact.emails.contains(&"home@example.com".to_string()));
        assert!(contact.emails.contains(&"other@example.com".to_string()));

        // Should capture all phone numbers
        assert_eq!(contact.phones.len(), 3);
        assert!(contact.phones.contains(&"+1-555-123-4567".to_string()));
        assert!(contact.phones.contains(&"+1-555-987-6543".to_string()));
        assert!(contact.phones.contains(&"+1-555-555-5555".to_string()));
    }

    #[test]
    fn test_contact_with_multiple_urls_and_categories() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:multi-url-category-contact
FN:Multi URL Category Person
URL:https://example.com
URL:https://company.com/profile
URL:https://personal-blog.com
CATEGORIES:Business
CATEGORIES:Personal,Friend
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        // Should capture all URLs
        assert_eq!(contact.urls.len(), 3);
        assert!(contact.urls.contains(&"https://example.com".to_string()));
        assert!(contact
            .urls
            .contains(&"https://company.com/profile".to_string()));
        assert!(contact
            .urls
            .contains(&"https://personal-blog.com".to_string()));

        // Should capture all categories (might be flattened or separate)
        assert!(!contact.categories.is_empty());
        // Categories could be parsed as separate items or comma-separated
        // Let's just verify we got some categories
        assert!(contact.categories.len() >= 2);
    }

    #[test]
    fn test_contact_with_multiple_addresses() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:multi-address-contact
FN:Multi Address Person
ADR;TYPE=WORK:;;123 Work St;Business City;CA;12345;USA
ADR;TYPE=HOME:;;456 Home Ave;Residential City;NY;67890;USA
ADR;TYPE=OTHER:;;789 Other Rd;Third City;TX;54321;USA
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "multi-address-contact");

        // Should capture all addresses
        assert_eq!(contact.addresses.len(), 3);
        // Addresses are structured fields, so we just verify we got the right count
        // and that they're not empty
        for address in &contact.addresses {
            assert!(!address.is_empty());
        }
    }

    #[test]
    fn test_contact_with_empty_and_mixed_multi_values() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:mixed-multi-contact
FN:Mixed Values Person
EMAIL:valid@example.com
EMAIL:
EMAIL:another@example.com
TEL:+1-555-123-4567
TEL:
URL:https://valid.com
URL:
CATEGORIES:Valid,Category
CATEGORIES:
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "mixed-multi-contact");

        // Should only capture non-empty values
        assert_eq!(contact.emails.len(), 2);
        assert!(contact.emails.contains(&"valid@example.com".to_string()));
        assert!(contact.emails.contains(&"another@example.com".to_string()));

        assert_eq!(contact.phones.len(), 1);
        assert!(contact.phones.contains(&"+1-555-123-4567".to_string()));

        assert_eq!(contact.urls.len(), 1);
        assert!(contact.urls.contains(&"https://valid.com".to_string()));
    }

    #[test]
    fn test_contact_with_maximum_realistic_fields() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:maximum-contact
FN:Maximum Fields Person
N:Person;Maximum;Fields;Dr.;Jr.
NICKNAME:Max
ORG:Big Corporation;Engineering Department
TITLE:Senior Software Engineer
ROLE:Tech Lead
EMAIL;TYPE=WORK:max@company.com
EMAIL;TYPE=HOME:max@personal.com
EMAIL;TYPE=OTHER:max.person@example.org
EMAIL:maxfields@temp.com
EMAIL:contact@maxperson.dev
TEL;TYPE=WORK:+1-555-123-4567
TEL;TYPE=HOME:+1-555-987-6543
TEL;TYPE=CELL:+1-555-555-5555
TEL;TYPE=FAX:+1-555-111-2222
URL:https://maxperson.dev
URL:https://github.com/maxperson
URL:https://linkedin.com/in/maxperson
URL:https://company.com/team/max
CATEGORIES:Developer,Manager,OpenSource,Speaker
BDAY:1985-03-15T00:00:00Z
NOTE:Experienced software engineer with expertise in Rust and Python. Loves open source and public speaking.
LANG:en
LANG:es
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "maximum-contact");
        assert_eq!(contact.full_name, Some("Maximum Fields Person".to_string()));
        assert!(contact.name.is_some());
        assert_eq!(contact.nickname, Some("Max".to_string()));
        assert_eq!(
            contact.organization,
            Some("Big Corporation;Engineering Department".to_string())
        );
        assert_eq!(contact.title, Some("Senior Software Engineer".to_string()));
        assert_eq!(contact.role, Some("Tech Lead".to_string()));

        // Multiple emails
        assert_eq!(contact.emails.len(), 5);

        // Multiple phones
        assert_eq!(contact.phones.len(), 4);

        // Multiple URLs
        assert_eq!(contact.urls.len(), 4);

        // Categories
        assert!(!contact.categories.is_empty());

        // Optional fields should be populated
        assert!(contact.birthday.is_some());
        assert!(contact.note.is_some());

        // Verify birthday parsing
        if let Some(birthday) = contact.birthday {
            assert_eq!(birthday.year(), 1985);
            assert_eq!(birthday.month(), 3);
            assert_eq!(birthday.day(), 15);
        }
    }

    #[test]
    fn test_contact_with_various_date_fields() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:date-fields-contact
FN:Date Fields Person
BDAY:1990-01-15T00:00:00Z
ANNIVERSARY:2015-06-20T00:00:00Z
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "date-fields-contact");

        // Verify birthday parsing
        if let Some(birthday) = contact.birthday {
            assert_eq!(birthday.year(), 1990);
            assert_eq!(birthday.month(), 1);
            assert_eq!(birthday.day(), 15);
        }

        // Verify anniversary parsing
        assert!(contact.anniversary.is_some());
    }

    // Unicode and special character tests
    #[test]
    fn test_contact_with_international_names_and_addresses() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:international-contact
FN:François José María O'Connor-Smith
N:O'Connor-Smith;Français José María;;;
ORG:Société Internationale Müller & Håkansson Inc.
TITLE:Développeur Senior / シニア・デベロッパー
EMAIL:françois@müller-company.com
EMAIL:jose.maria@håkansson.no
TEL:+33-1-42-86-83-02
TEL:+47-22-12-34-56
ADR;TYPE=WORK:;;123 Rue de la Paix;Paris;Île-de-France;75001;France
ADR;TYPE=HOME:;;Østergade 45;København;Hovedstaden;1100;Danmark
URL:https://müller-håkansson.com
CATEGORIES:international,développeur,開発者
NOTE:Speaks français, english, español, 日本語, and norsk
LANG:fr
LANG:en
LANG:es
LANG:ja
LANG:no
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "international-contact");
        assert_eq!(
            contact.full_name,
            Some("François José María O'Connor-Smith".to_string())
        );
        assert_eq!(
            contact.organization,
            Some("Société Internationale Müller & Håkansson Inc.".to_string())
        );
        assert_eq!(
            contact.title,
            Some("Développeur Senior / シニア・デベロッパー".to_string())
        );

        // Email with international domain
        assert!(contact
            .emails
            .contains(&"françois@müller-company.com".to_string()));
        assert!(contact
            .emails
            .contains(&"jose.maria@håkansson.no".to_string()));

        // International phone numbers
        assert!(contact.phones.contains(&"+33-1-42-86-83-02".to_string()));
        assert!(contact.phones.contains(&"+47-22-12-34-56".to_string()));

        // Addresses with international characters
        assert_eq!(contact.addresses.len(), 2);

        // URL with international domain
        assert!(contact
            .urls
            .contains(&"https://müller-håkansson.com".to_string()));

        // Categories with mixed languages
        assert!(!contact.categories.is_empty());

        // Note with multiple languages
        assert!(contact.note.as_ref().unwrap().contains("français"));
        assert!(contact.note.as_ref().unwrap().contains("日本語"));
    }

    #[test]
    fn test_fields_with_emoji_and_modern_unicode() {
        let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
UID:emoji-contact
FN:Alex 🚀 Developer
ORG:Tech Startup 💻🌟
TITLE:Senior Dev 👨‍💻 & Team Lead 👑
EMAIL:alex@startup🚀.com
NOTE:Love coding 💻, coffee ☕, and travel 🌍. Currently working on AI 🤖 projects!
CATEGORIES:dev,startup,🚀,💻,AI
URL:https://github.com/alex-dev-🚀
END:VCARD"#;

        let contact = Contact::from_str(vcard_data).unwrap();

        assert_eq!(contact.uid, "emoji-contact");
        assert_eq!(contact.full_name, Some("Alex 🚀 Developer".to_string()));
        assert_eq!(contact.organization, Some("Tech Startup 💻🌟".to_string()));
        assert_eq!(
            contact.title,
            Some("Senior Dev 👨‍💻 & Team Lead 👑".to_string())
        );

        // Note that email with emoji domain might not be valid in practice,
        // but our parser should handle it gracefully
        assert!(contact.emails.contains(&"alex@startup🚀.com".to_string()));

        // Note with emojis
        assert!(contact.note.as_ref().unwrap().contains("💻"));
        assert!(contact.note.as_ref().unwrap().contains("🤖"));

        // Categories with emojis
        assert!(!contact.categories.is_empty());

        // URL with emoji
        assert!(contact
            .urls
            .contains(&"https://github.com/alex-dev-🚀".to_string()));
    }
}
