# sysctl-conf-parser

Parse sysctl.conf-style configuration files into nested maps (dictionary structure).

## Grammar (sysctl.conf(5))

- `key = value` — leading/trailing whitespace is trimmed
- Blank lines and lines starting with `#` or `;` are ignored
- A leading `-` means "ignore failure"; the line is still parsed after stripping `-`

Dot notation in keys creates nested maps: `log.file = /var/log/app.log` becomes `log: { file: "/var/log/app.log" }`.

## Schema validation

Schema files use the same grammar as sysctl.conf(5): `key = type` per line. Supported types: `string`, `bool`, `integer`, `float` (aliases: `boolean`, `int`, `number`). Dot notation is supported.

Example schema (`sample.schema`):

```
endpoint = string
debug = bool
log.file = string
retry = integer
```

- Every key in the config must be defined in the schema.
- Values are validated against the schema type (e.g. `debug = true` is valid for `bool`; `retry = abc` is invalid for `integer`).
- Validation errors report the key, expected type, and actual value.

## Usage (library)

```rust
use sysctl_conf::{parse_str, load_file, load_schema, validate, Value};

// Parse config
let map = parse_str(input)?;
// or
let map = load_file("config.conf")?;

// Optional: validate against schema
let schema = load_schema("schema.conf")?;
validate(&map, &schema)?;
```

## Example

```bash
# Parse only
cargo run --example parse_file -- examples/sample1.conf

# Parse and validate with schema
cargo run --example parse_file -- examples/sample1.conf examples/sample.schema
```

## Tests

```bash
cargo test
```
