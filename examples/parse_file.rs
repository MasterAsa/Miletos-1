//! Example: load a sysctl.conf-style file and print the parsed map.
//! Optionally validate against a schema file.
//!
//! Usage: cargo run --example parse_file -- <config_path> [schema_path]
//! Example: cargo run --example parse_file -- examples/sample1.conf
//! Example: cargo run --example parse_file -- examples/sample1.conf examples/sample.schema

use std::env;
use std::process;
use sysctl_conf::{load_file, load_schema, validate, Value};

fn print_value(v: &Value, indent: usize) {
    let pad = "  ".repeat(indent);
    match v {
        Value::String(s) => println!("{pad}\"{s}\""),
        Value::Map(m) => {
            let n = m.len();
            for (i, (k, v)) in m.iter().enumerate() {
                let comma = if i < n - 1 { "," } else { "" };
                print!("{pad}\"{k}\": ");
                match v {
                    Value::String(s) => println!("\"{s}\"{comma}"),
                    Value::Map(_) => {
                        println!("{{");
                        print_value(v, indent + 1);
                        println!("{pad}}}{comma}");
                    }
                }
            }
        }
    }
}

fn main() {
    let config_path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: parse_file <config_path> [schema_path]");
        process::exit(1);
    });
    let schema_path = env::args().nth(2);

    let root = match load_file(&config_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error loading config: {}", e);
            process::exit(1);
        }
    };

    if let Some(schema_path) = schema_path {
        let schema = match load_schema(&schema_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error loading schema: {}", e);
                process::exit(1);
            }
        };
        if let Err(e) = validate(&root, &schema) {
            eprintln!("validation error: {}", e);
            process::exit(1);
        }
    }

    println!("{{");
    let n = root.len();
    for (i, (k, v)) in root.iter().enumerate() {
        let comma = if i < n - 1 { "," } else { "" };
        print!("  \"{k}\": ");
        match v {
            Value::String(s) => println!("\"{s}\"{comma}"),
            Value::Map(_) => {
                println!("{{");
                print_value(v, 2);
                println!("  }}{comma}");
            }
        }
    }
    println!("}}");
}
