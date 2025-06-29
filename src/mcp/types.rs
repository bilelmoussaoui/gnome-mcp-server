use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub result: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    pub uri: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub mime_type: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceContent {
    pub uri: &'static str,
    pub mime_type: &'static str,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

pub trait ResourceProvider {
    const URI: &'static str;
    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    const MIME_TYPE: &'static str = "application/json";

    fn resource() -> Resource {
        Resource {
            uri: Self::URI,
            name: Self::NAME,
            description: Self::DESCRIPTION,
            mime_type: Self::MIME_TYPE,
        }
    }

    async fn get_content(&self) -> Result<ResourceContent>;
}

pub trait ToolProvider {
    const NAME: &'static str;
    const DESCRIPTION: &'static str;

    fn get_tool_definition() -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME,
            description: Self::DESCRIPTION,
            input_schema: Self::input_schema(),
        }
    }

    fn input_schema() -> serde_json::Value;
    async fn execute(&self, arguments: &serde_json::Value) -> Result<serde_json::Value>;
}
