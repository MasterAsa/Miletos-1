//! Example: load a sysctl.conf-style file and print the parsed map.
//!
//! Usage: cargo run --example parse_file -- <path>
//! Example: cargo run --example parse_file -- examples/sample1.conf

use std::env;
use std::process;
use sysctl_conf::{load_file, Value};

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
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: parse_file <path>");
        process::exit(1);
    });

    let root = match load_file(&path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

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
