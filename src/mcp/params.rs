#[macro_export]
macro_rules! tool_params {
    // Mixed required and optional parameters with semicolon separator
    (
        $struct_name:ident,
        $(required($name:ident: $type:ident, $desc:expr)),* $(,)?
        ; $(optional($opt_name:ident: $opt_type:ident = $default:expr, $opt_desc:expr)),* $(,)?
    ) => {
        #[derive(Debug)]
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

    // Mixed required and optional parameters without semicolon separator
    (
        $struct_name:ident,
        $(required($name:ident: $type:ident, $desc:expr)),* $(,)?
        $(optional($opt_name:ident: $opt_type:ident = $default:expr, $opt_desc:expr)),* $(,)?
    ) => {
        #[derive(Debug)]
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
        #[derive(Debug)]
        pub struct $struct_name {
            $(pub $name: tool_params!(@rust_type $type),)*
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
                    },
                    "required": [$(stringify!($name)),*]
                })
            }

            fn extract_params(arguments: &serde_json::Value) -> anyhow::Result<Self> {
                Ok(Self {
                    $(
                        $name: tool_params!(@extract_required $type, arguments, stringify!($name))?,
                    )*
                })
            }
        }
    };

    // Only optional parameters
    (
        $struct_name:ident,
        ; $(optional($opt_name:ident: $opt_type:ident = $default:expr, $opt_desc:expr)),* $(,)?
    ) => {
        #[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use crate::mcp::ToolParams;
    use serde_json::json;

    // Test struct with only required parameters
    tool_params! {
        RequiredOnlyParams,
        required(message: string, "A required message"),
        required(urgent: bool, "A required urgency flag")
    }

    // Test struct with only optional parameters
    tool_params! {
        OptionalOnlyParams,
        optional(timeout: i64 = 5000, "Timeout in milliseconds"),
        optional(debug: bool = false, "Debug mode flag")
    }

    // Test struct with required and optional parameters
    tool_params! {
        TestParams,
        required(name: string, "A required name parameter"),
        required(enabled: bool, "A required boolean parameter"),
        optional(count: i64 = 10, "An optional integer parameter")
    }

    #[test]
    fn test_mixed_params_schema_generation() {
        let schema = TestParams::input_schema();
        let expected = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "A required name parameter"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "A required boolean parameter"
                },
                "count": {
                    "type": "integer",
                    "description": "An optional integer parameter"
                }
            },
            "required": ["name", "enabled"]
        });
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_required_only_schema_generation() {
        let schema = RequiredOnlyParams::input_schema();
        let expected = json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "A required message"
                },
                "urgent": {
                    "type": "boolean",
                    "description": "A required urgency flag"
                }
            },
            "required": ["message", "urgent"]
        });
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_optional_only_schema_generation() {
        let schema = OptionalOnlyParams::input_schema();
        let expected = json!({
            "type": "object",
            "properties": {
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds"
                },
                "debug": {
                    "type": "boolean",
                    "description": "Debug mode flag"
                }
            },
            "required": []
        });
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_successful_parameter_extraction() {
        let input = json!({
            "name": "test",
            "enabled": true,
            "count": 42
        });

        let params = TestParams::extract_params(&input).unwrap();
        assert_eq!(params.name, "test");
        assert_eq!(params.enabled, true);
        assert_eq!(params.count, 42);
    }

    #[test]
    fn test_parameter_extraction_with_defaults() {
        let input = json!({
            "name": "test",
            "enabled": false
            // count not provided, should use default
        });

        let params = TestParams::extract_params(&input).unwrap();
        assert_eq!(params.name, "test");
        assert_eq!(params.enabled, false);
        assert_eq!(params.count, 10); // default value
    }

    #[test]
    fn test_missing_required_parameter_error() {
        let input = json!({
            "name": "test"
            // missing required "enabled" parameter
        });

        let result = TestParams::extract_params(&input);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Missing required parameter: enabled"));
    }

    #[test]
    fn test_optional_only_extraction_with_defaults() {
        let input = json!({});

        let params = OptionalOnlyParams::extract_params(&input).unwrap();
        assert_eq!(params.timeout, 5000);
        assert_eq!(params.debug, false);
    }

    #[test]
    fn test_optional_only_extraction_with_values() {
        let input = json!({
            "timeout": 1000,
            "debug": true
        });

        let params = OptionalOnlyParams::extract_params(&input).unwrap();
        assert_eq!(params.timeout, 1000);
        assert_eq!(params.debug, true);
    }

    #[test]
    fn test_type_conversion_errors() {
        let input = json!({
            "name": "test",
            "enabled": "not_a_boolean" // wrong type
        });

        let result = TestParams::extract_params(&input);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Missing required parameter: enabled"));
    }

    #[test]
    fn test_string_default_with_expression() {
        tool_params! {
            StringDefaultParams,
            ; optional(prefix: string = "default".to_string(), "A string with default")
        }

        let input = json!({});
        let params = StringDefaultParams::extract_params(&input).unwrap();
        assert_eq!(params.prefix, "default");
    }

    #[test]
    fn test_partial_optional_override() {
        let input = json!({
            "name": "test",
            "enabled": true,
            "count": 99
        });

        let params = TestParams::extract_params(&input).unwrap();
        assert_eq!(params.name, "test");
        assert_eq!(params.enabled, true);
        assert_eq!(params.count, 99);
    }
}
