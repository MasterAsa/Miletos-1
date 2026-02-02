//! Schema validation for sysctl.conf-style configs.
//!
//! Schema files use the same grammar as sysctl.conf(5): `key = type` per line.
//! Supported types: `string`, `bool`, `integer`, `float`.
//! Dot notation is supported: `log.file = string`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::{parse_str, Value};

/// Expected type for a config key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaType {
    String,
    Bool,
    Integer,
    Float,
}

impl SchemaType {
    fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_lowercase().as_str() {
            "string" => Some(SchemaType::String),
            "bool" | "boolean" => Some(SchemaType::Bool),
            "integer" | "int" => Some(SchemaType::Integer),
            "float" | "number" => Some(SchemaType::Float),
            _ => None,
        }
    }

    /// Returns true if the raw string is valid for this type.
    fn check_value(&self, raw: &str) -> bool {
        match self {
            SchemaType::String => true,
            SchemaType::Bool => {
                let s = raw.trim().to_lowercase();
                matches!(s.as_str(), "true" | "false" | "1" | "0" | "yes" | "no")
            }
            SchemaType::Integer => raw.trim().parse::<i64>().is_ok(),
            SchemaType::Float => raw.trim().parse::<f64>().is_ok(),
        }
    }
}

/// Parsed schema: dotted key path -> expected type.
pub type Schema = HashMap<String, SchemaType>;

/// Parses a schema string (same grammar as sysctl.conf: `key = type`).
/// Returns a flat map of dotted paths to schema types.
pub fn parse_schema_str(input: &str) -> Result<Schema, SchemaParseError> {
    let parsed = parse_str(input).map_err(|e| SchemaParseError::Parse(e))?;
    flatten_to_schema(parsed)
}

/// Loads and parses a schema file.
pub fn load_schema(path: impl AsRef<Path>) -> Result<Schema, SchemaLoadError> {
    let content = fs::read_to_string(path.as_ref()).map_err(SchemaLoadError::Io)?;
    parse_schema_str(&content).map_err(SchemaLoadError::Parse)
}

fn flatten_to_schema(parsed: HashMap<String, Value>) -> Result<Schema, SchemaParseError> {
    let mut schema = Schema::new();
    flatten_to_schema_impl(&parsed, "", &mut schema)?;
    Ok(schema)
}

fn flatten_to_schema_impl(
    map: &HashMap<String, Value>,
    prefix: &str,
    out: &mut Schema,
) -> Result<(), SchemaParseError> {
    for (k, v) in map {
        let path = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{}.{}", prefix, k)
        };
        match v {
            Value::String(type_name) => {
                let schema_type = SchemaType::from_name(type_name).ok_or_else(|| {
                    SchemaParseError::UnknownType {
                        key: path.clone(),
                        type_name: type_name.clone(),
                    }
                })?;
                out.insert(path, schema_type);
            }
            Value::Map(m) => flatten_to_schema_impl(m, &path, out)?,
        }
    }
    Ok(())
}

/// Validates a parsed config against a schema.
/// - Every key in config must be defined in schema.
/// - Every value must parse as the schema type (string, bool, integer, float).
pub fn validate(config: &HashMap<String, Value>, schema: &Schema) -> Result<(), SchemaValidationError> {
    validate_impl(config, "", schema)
}

fn validate_impl(
    config: &HashMap<String, Value>,
    prefix: &str,
    schema: &Schema,
) -> Result<(), SchemaValidationError> {
    for (k, v) in config {
        let path = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{}.{}", prefix, k)
        };
        match v {
            Value::String(raw) => {
                let expected = schema.get(&path).ok_or_else(|| SchemaValidationError::UnknownKey {
                    key: path.clone(),
                })?;
                let expected_name = match expected {
                    SchemaType::String => "string",
                    SchemaType::Bool => "bool",
                    SchemaType::Integer => "integer",
                    SchemaType::Float => "float",
                };
                if !expected.check_value(raw) {
                    return Err(SchemaValidationError::InvalidType {
                        key: path,
                        expected: expected_name.to_string(),
                        value: raw.clone(),
                    });
                }
            }
            Value::Map(m) => validate_impl(m, &path, schema)?,
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum SchemaParseError {
    Parse(crate::ParseError),
    UnknownType { key: String, type_name: String },
}

impl std::fmt::Display for SchemaParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaParseError::Parse(e) => write!(f, "parse: {}", e),
            SchemaParseError::UnknownType { key, type_name } => {
                write!(f, "schema key '{}': unknown type '{}'", key, type_name)
            }
        }
    }
}

impl std::error::Error for SchemaParseError {}

#[derive(Debug)]
pub enum SchemaLoadError {
    Io(std::io::Error),
    Parse(SchemaParseError),
}

impl std::fmt::Display for SchemaLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaLoadError::Io(e) => write!(f, "io: {}", e),
            SchemaLoadError::Parse(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for SchemaLoadError {}

#[derive(Debug)]
pub enum SchemaValidationError {
    UnknownKey { key: String },
    InvalidType {
        key: String,
        expected: String,
        value: String,
    },
}

impl std::fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaValidationError::UnknownKey { key } => {
                write!(f, "validation error: key '{}' is not defined in schema", key)
            }
            SchemaValidationError::InvalidType {
                key,
                expected,
                value,
            } => write!(
                f,
                "validation error: key '{}' expected type '{}', got value '{}'",
                key, expected, value
            ),
        }
    }
}

impl std::error::Error for SchemaValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_str;

    #[test]
    fn parse_schema_example() {
        let input = r#"
endpoint = string
debug = bool
log.file = string
retry = integer
"#;
        let schema = parse_schema_str(input).unwrap();
        assert_eq!(schema.get("endpoint"), Some(&SchemaType::String));
        assert_eq!(schema.get("debug"), Some(&SchemaType::Bool));
        assert_eq!(schema.get("log.file"), Some(&SchemaType::String));
        assert_eq!(schema.get("retry"), Some(&SchemaType::Integer));
    }

    #[test]
    fn validate_ok() {
        let schema_input = "endpoint = string\ndebug = bool\nlog.file = string\n";
        let config_input = r#"
endpoint = localhost:3000
debug = true
log.file = /var/log/console.log
"#;
        let schema = parse_schema_str(schema_input).unwrap();
        let config = parse_str(config_input).unwrap();
        assert!(validate(&config, &schema).is_ok());
    }

    #[test]
    fn validate_unknown_key() {
        let schema_input = "endpoint = string\n";
        let config_input = "endpoint = localhost\nunknown = value\n";
        let schema = parse_schema_str(schema_input).unwrap();
        let config = parse_str(config_input).unwrap();
        let err = validate(&config, &schema).unwrap_err();
        match &err {
            SchemaValidationError::UnknownKey { key } => assert_eq!(key, "unknown"),
            _ => panic!("expected UnknownKey"),
        }
    }

    #[test]
    fn validate_invalid_type_bool() {
        let schema_input = "debug = bool\n";
        let config_input = "debug = notabool\n";
        let schema = parse_schema_str(schema_input).unwrap();
        let config = parse_str(config_input).unwrap();
        let err = validate(&config, &schema).unwrap_err();
        match &err {
            SchemaValidationError::InvalidType { key, expected, value } => {
                assert_eq!(key, "debug");
                assert_eq!(expected, "bool");
                assert_eq!(value, "notabool");
            }
            _ => panic!("expected InvalidType"),
        }
    }

    #[test]
    fn validate_invalid_type_integer() {
        let schema_input = "retry = integer\n";
        let config_input = "retry = abc\n";
        let schema = parse_schema_str(schema_input).unwrap();
        let config = parse_str(config_input).unwrap();
        let err = validate(&config, &schema).unwrap_err();
        match &err {
            SchemaValidationError::InvalidType { expected, .. } => assert_eq!(expected, "integer"),
            _ => panic!("expected InvalidType"),
        }
    }
}