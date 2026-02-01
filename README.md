# sysctl-conf-parser

Parse sysctl.conf-style configuration files into nested maps (dictionary structure).

## Grammar (sysctl.conf(5))

- `key = value` — leading/trailing whitespace is trimmed
- Blank lines and lines starting with `#` or `;` are ignored
- A leading `-` means "ignore failure"; the line is still parsed after stripping `-`

Dot notation in keys creates nested maps: `log.file = /var/log/app.log` becomes `log: { file: "/var/log/app.log" }`.

## Usage (library)

```rust
use sysctl_conf::{parse_str, load_file, Value};

// From string
let input = r#"
endpoint = localhost:3000
log.file = /var/log/console.log
"#;
let map = parse_str(input)?;

// From file
let map = load_file("config.conf")?;

// Access: map is HashMap<String, Value>
// Value is either String(s) or Map(HashMap<String, Value>)
```

## Example

```bash
cargo run --example parse_file -- examples/sample1.conf
```

## Tests

```bash
cargo test
```
