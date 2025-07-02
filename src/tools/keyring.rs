use std::collections::HashMap;

use anyhow::Result;
use oo7::dbus::Service;
use serde_json::{json, Value};

use crate::{mcp::ToolProvider, tool_params};

#[derive(Default)]
pub struct Keyring;

tool_params! {
    KeyringParams,
    required(action: string, "Action to perform: 'store', 'retrieve', 'delete'"),
    optional(label: string, "Human-readable label for the secret (required for store action)"),
    optional(secret: string, "The secret value to store (required for store action)"),
    optional(attributes: string, "JSON object of key-value attributes for categorizing/searching secrets (e.g. {\"application\": \"myapp\", \"username\": \"user\"})")
}

impl ToolProvider for Keyring {
    const NAME: &'static str = "keyring_management";
    const DESCRIPTION: &'static str =
        "Manage secrets in the GNOME Keyring. Actions: store, retrieve, delete";
    type Params = KeyringParams;

    async fn execute_with_params(&self, params: Self::Params) -> Result<Value> {
        match params.action.as_str() {
            "store" => {
                let label = params
                    .label
                    .ok_or_else(|| anyhow::anyhow!("label required for store action"))?;
                let secret = params
                    .secret
                    .ok_or_else(|| anyhow::anyhow!("secret required for store action"))?;
                let attributes = params.attributes.unwrap_or_else(|| "{}".to_string());
                store_secret(label, secret, attributes).await
            }
            "retrieve" => {
                let attributes = params.attributes.unwrap_or_else(|| "{}".to_string());
                retrieve_secret(attributes).await
            }
            "delete" => {
                let attributes = params.attributes.unwrap_or_else(|| "{}".to_string());
                delete_secret(attributes).await
            }
            _ => Err(anyhow::anyhow!(
                "Unknown action: {}. Available: store, retrieve, delete",
                params.action
            )),
        }
    }
}

async fn store_secret(label: String, secret: String, attributes: String) -> Result<Value> {
    let service = Service::new().await?;
    let collection = service.default_collection().await?;

    // Parse attributes from JSON string
    let attributes: HashMap<String, String> = if attributes.trim().is_empty() || attributes == "{}"
    {
        HashMap::new()
    } else {
        serde_json::from_str(&attributes)
            .map_err(|e| anyhow::anyhow!("Invalid attributes JSON: {}", e))?
    };

    collection
        .create_item(
            &label,
            &attributes,
            secret.as_bytes(),
            true, // replace if exists
            None, // window_id
        )
        .await?;

    Ok(json!({
        "success": true,
        "message": format!("Secret '{}' stored successfully", label)
    }))
}

async fn retrieve_secret(attributes: String) -> Result<Value> {
    let service = Service::new().await?;
    let collection = service.default_collection().await?;

    // Parse search attributes from JSON string
    let search_attributes: HashMap<String, String> =
        if attributes.trim().is_empty() || attributes == "{}" {
            return Ok(json!({
                "error": "Attributes cannot be empty"
            }));
        } else {
            serde_json::from_str(&attributes)
                .map_err(|e| anyhow::anyhow!("Invalid attributes JSON: {}", e))?
        };

    let items = collection.search_items(&search_attributes).await?;

    if let Some(item) = items.first() {
        let secret = item.secret().await?;
        let secret_str = String::from_utf8_lossy(&secret);

        Ok(json!({
            "success": true,
            "secret": secret_str,
            "label": item.label().await?,
            "attributes": item.attributes().await?
        }))
    } else {
        Ok(json!({
            "error": "Secret not found"
        }))
    }
}

async fn delete_secret(attributes: String) -> Result<Value> {
    let service = Service::new().await?;
    let collection = service.default_collection().await?;

    // Parse search attributes from JSON string
    let search_attributes: HashMap<String, String> =
        if attributes.trim().is_empty() || attributes == "{}" {
            return Ok(json!({
                "error": "Attributes cannot be empty"
            }));
        } else {
            serde_json::from_str(&attributes)
                .map_err(|e| anyhow::anyhow!("Invalid attributes JSON: {}", e))?
        };

    let items = collection.search_items(&search_attributes).await?;

    if let Some(item) = items.first() {
        let item_label = item.label().await?;
        item.delete(None).await?;

        Ok(json!({
            "success": true,
            "message": format!("Secret '{}' deleted successfully", item_label)
        }))
    } else {
        Ok(json!({
            "error": "Secret not found"
        }))
    }
}
