/// Simple macro to generate both JSON schema and parameter extraction
#[macro_export]
macro_rules! tool_params {
    (
        $struct_name:ident,
        $(required($name:ident: $type:ident, $desc:expr)),* $(,)?
        $(; optional($opt_name:ident: $opt_type:ident = $default:expr, $opt_desc:expr))* $(,)?
    ) => {
        pub struct $struct_name {
            $(pub $name: tool_params!(@rust_type $type),)*
            $(pub $opt_name: tool_params!(@rust_type $opt_type),)*
        }

        impl $crate::mcp::ToolParams for $struct_name {
            fn input_schema() -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        $(
                            stringify!($name): {
                                "type": tool_params!(@json_type $type),
                                "description": $desc
                            },
                        )*
                        $(
                            stringify!($opt_name): {
                                "type": tool_params!(@json_type $opt_type),
                                "description": $opt_desc
                            },
                        )*
                    },
                    "required": [$(stringify!($name)),*]
                })
            }

            fn extract_params(arguments: &serde_json::Value) -> anyhow::Result<Self> {
                Ok(Self {
                    $(
                        $name: tool_params!(@extract_required $type, arguments, stringify!($name))?,
                    )*
                    $(
                        $opt_name: tool_params!(@extract_optional $opt_type, arguments, stringify!($opt_name), $default),
                    )*
                })
            }
        }
    };

    // Only required parameters
    (
        $struct_name:ident,
        $(required($name:ident: $type:ident, $desc:expr)),* $(,)?
    ) => {
        tool_params! {
            $struct_name,
            $(required($name: $type, $desc)),*;
        }
    };

    // Only optional parameters
    (
        $struct_name:ident,
        ; $(optional($opt_name:ident: $opt_type:ident = $default:expr, $opt_desc:expr)),* $(,)?
    ) => {
        pub struct $struct_name {
            $(pub $opt_name: tool_params!(@rust_type $opt_type),)*
        }

        impl $crate::mcp::ToolParams for $struct_name {
            fn input_schema() -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        $(
                            stringify!($opt_name): {
                                "type": tool_params!(@json_type $opt_type),
                                "description": $opt_desc
                            },
                        )*
                    },
                    "required": []
                })
            }

            fn extract_params(arguments: &serde_json::Value) -> anyhow::Result<Self> {
                Ok(Self {
                    $(
                        $opt_name: tool_params!(@extract_optional $opt_type, arguments, stringify!($opt_name), $default),
                    )*
                })
            }
        }
    };

    // Type mappings
    (@json_type string) => { "string" };
    (@json_type bool) => { "boolean" };
    (@json_type f64) => { "number" };
    (@json_type i64) => { "integer" };

    (@rust_type string) => { String };
    (@rust_type bool) => { bool };
    (@rust_type f64) => { f64 };
    (@rust_type i64) => { i64 };

    // Extraction
    (@extract_required string, $args:expr, $name:expr) => {
        $args
            .get($name)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: {}", $name))
            .map(|s| s.to_string())
    };
    (@extract_required bool, $args:expr, $name:expr) => {
        $args
            .get($name)
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: {}", $name))
    };

    (@extract_optional string, $args:expr, $name:expr, $default:expr) => {
        $args
            .get($name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| $default.to_string())
    };
    (@extract_optional bool, $args:expr, $name:expr, $default:expr) => {
        $args.get($name).and_then(|v| v.as_bool()).unwrap_or($default)
    };
    (@extract_optional f64, $args:expr, $name:expr, $default:expr) => {
        $args.get($name).and_then(|v| v.as_f64()).unwrap_or($default)
    };
    (@extract_optional i64, $args:expr, $name:expr, $default:expr) => {
        $args.get($name).and_then(|v| v.as_i64()).unwrap_or($default)
    };
}
