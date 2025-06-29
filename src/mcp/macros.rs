macro_rules! register_providers {
    (
        resources: [ $($resource:path),* $(,)? ],
        tools: [ $($tool:path),* $(,)? ]
    ) => {
        pub fn list_resources() -> Vec<crate::mcp::Resource> {
            vec![ $( <$resource>::resource() ),* ]
        }

        pub async fn resource_for_uri(uri: &str) -> anyhow::Result<crate::mcp::ResourceContent> {
            $(
                if <$resource>::URI == uri {
                    return <$resource>::default().get_content().await;
                }
            )*
            anyhow::bail!("Unsupported URI {uri}")
        }

        pub fn list_tools() -> Vec<crate::mcp::ToolDefinition> {
            vec![ $( <$tool>::get_tool_definition() ),* ]
        }

        pub async fn execute_tool(name: &str, arguments: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
            $(
                if <$tool>::NAME == name {
                    return <$tool>::default().execute(arguments).await;
                }
            )*
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    };
}

pub(crate) use register_providers;
