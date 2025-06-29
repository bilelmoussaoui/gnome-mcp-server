macro_rules! register_providers {
    (
        resources: [ $($resource:path),* $(,)? ],
        tools: [ $($tool:path),* $(,)? ]
    ) => {
        pub fn list_resources() -> Vec<crate::mcp::Resource> {
            let mut resources = Vec::new();
            $(
                if crate::config::CONFIG.is_resource_enabled::<$resource>() {
                    resources.push(<$resource>::resource());
                }
            )*
            resources
        }

        pub async fn resource_for_uri(uri: &str) -> anyhow::Result<crate::mcp::ResourceContent> {
            $(
                if <$resource>::URI == uri && crate::config::CONFIG.is_resource_enabled::<$resource>() {
                    return <$resource>::default().get_content().await;
                }
            )*
            anyhow::bail!("Unsupported URI {uri}")
        }

        pub fn list_tools() -> Vec<crate::mcp::ToolDefinition> {
            let mut tools = Vec::new();
            $(
                if crate::config::CONFIG.is_tool_enabled::<$tool>() {
                    tools.push(<$tool>::get_tool_definition());
                }
            )*
            tools
        }

        pub async fn execute_tool(name: &str, arguments: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
            $(
                if <$tool>::NAME == name && crate::config::CONFIG.is_tool_enabled::<$tool>() {
                    return <$tool>::default().execute(arguments).await;
                }
            )*
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    };
}

pub(crate) use register_providers;
