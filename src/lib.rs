//! Parse sysctl.conf-style files into nested maps.
//!
//! Grammar (per sysctl.conf(5)):
//! - `key = value` (leading/trailing whitespace trimmed)
//! - Blank lines and lines starting with `#` or `;` are ignored
//! - A leading `-` means "ignore failure" (the `-` is stripped and the line is parsed)
//!
//! Dot notation in keys creates nested maps: `log.file = path` → `log: { file: "path" }`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A value is either a leaf string or a nested map (for dot-notation keys).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Map(HashMap<String, Value>),
}

/// Parses a sysctl.conf-style string and returns a top-level map.
/// Nested keys (e.g. `log.file`) are stored as nested maps.
pub fn parse_str(input: &str) -> Result<HashMap<String, Value>, ParseError> {
    let mut root: HashMap<String, Value> = HashMap::new();

    for (line_num, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        let line = line.strip_prefix('-').unwrap_or(line).trim();
        if line.is_empty() {
            continue;
        }

        let Some((key_part, value_part)) = line.split_once('=') else {
            return Err(ParseError {
                line: line_num + 1,
                message: "missing '='".into(),
            });
        };

        let key = key_part.trim();
        let value = value_part.trim();

        if key.is_empty() {
            return Err(ParseError {
                line: line_num + 1,
                message: "empty key".into(),
            });
        }

        set_nested(&mut root, key, Value::String(value.to_string()));
    }

    Ok(root)
}

/// Loads and parses a file. Returns the same structure as `parse_str`.
pub fn load_file(path: impl AsRef<Path>) -> Result<HashMap<String, Value>, LoadError> {
    let content = fs::read_to_string(path.as_ref()).map_err(LoadError::Io)?;
    parse_str(&content).map_err(LoadError::Parse)
}

/// Sets a possibly dotted key into a nested map. Creates intermediate maps as needed.
fn set_nested(root: &mut HashMap<String, Value>, key: &str, value: Value) {
    let parts: Vec<&str> = key.split('.').map(str::trim).collect();
    if parts.is_empty() {
        return;
    }
    if parts.len() == 1 {
        root.insert(parts[0].to_string(), value);
        return;
    }

    let (first, rest) = parts.split_at(1);
    let first = first[0];

    let entry = root
        .entry(first.to_string())
        .or_insert_with(|| Value::Map(HashMap::new()));

    let Value::Map(ref mut map) = entry else {
        // key conflict: already a string (e.g. "log" then "log.file")
        *entry = Value::Map(HashMap::new());
        if let Value::Map(ref mut m) = entry {
            set_nested_rest(m, rest, value);
        }
        return;
    };

    set_nested_rest(map, rest, value);
}

fn set_nested_rest(map: &mut HashMap<String, Value>, parts: &[&str], value: Value) {
    if parts.len() == 1 {
        map.insert(parts[0].to_string(), value);
        return;
    }

    let (first, rest) = parts.split_at(1);
    let first = first[0];

    let entry = map
        .entry(first.to_string())
        .or_insert_with(|| Value::Map(HashMap::new()));

    let Value::Map(ref mut nested) = entry else {
        *entry = Value::Map(HashMap::new());
        if let Value::Map(ref mut m) = entry {
            set_nested_rest(m, rest, value);
        }
        return;
    };

    set_nested_rest(nested, rest, value);
}

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(ParseError),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "io: {}", e),
            LoadError::Parse(e) => write!(f, "parse: {}", e),
        }
    }
}

impl std::error::Error for LoadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example1() {
        let input = r#"
endpoint = localhost:3000
debug = true
log.file = /var/log/console.log
"#;
        let got = parse_str(input).unwrap();
        let mut expected = HashMap::new();
        expected.insert(
            "endpoint".into(),
            Value::String("localhost:3000".into()),
        );
        expected.insert("debug".into(), Value::String("true".into()));
        let mut log = HashMap::new();
        log.insert("file".into(), Value::String("/var/log/console.log".into()));
        expected.insert("log".into(), Value::Map(log));
        assert_eq!(got, expected);
    }

    #[test]
    fn example2() {
        let input = r#"
endpoint = localhost:3000
# debug = true
log.file = /var/log/console.log
log.name = default.log
"#;
        let got = parse_str(input).unwrap();
        let mut expected = HashMap::new();
        expected.insert(
            "endpoint".into(),
            Value::String("localhost:3000".into()),
        );
        let mut log = HashMap::new();
        log.insert("file".into(), Value::String("/var/log/console.log".into()));
        log.insert("name".into(), Value::String("default.log".into()));
        expected.insert("log".into(), Value::Map(log));
        assert_eq!(got, expected);
    }

    #[test]
    fn blank_lines_and_comments() {
        let input = r#"

# comment
endpoint = localhost
; also comment
other = value
"#;
        let got = parse_str(input).unwrap();
        assert_eq!(
            got.get("endpoint"),
            Some(&Value::String("localhost".into()))
        );
        assert_eq!(
            got.get("other"),
            Some(&Value::String("value".into()))
        );
    }

    #[test]
    fn leading_minus_ignored() {
        let input = "-kernel.foo = bar";
        let got = parse_str(input).unwrap();
        let mut k = HashMap::new();
        k.insert("foo".into(), Value::String("bar".into()));
        assert_eq!(got.get("kernel"), Some(&Value::Map(k)));
    }
}
