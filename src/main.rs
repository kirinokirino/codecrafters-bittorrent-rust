use serde_bencode::{de, value::Value};

use std::env;

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    // i576805101e
    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        // println!("Logs from your program will appear here!");

        let encoded_value = &args[2];
        let decoded_value: Value = de::from_str(encoded_value).unwrap();
        display_value(&decoded_value);
        println!();
    } else {
        println!("unknown command: {}", args[1])
    }
}

fn display_value(value: &Value) {
    match value {
        Value::Bytes(bytes) => print!("\"{}\"", String::from_utf8_lossy(&bytes)),
        Value::Int(int) => print!("{}", int),
        Value::List(list) => {
            print!("[");
            for (i, value) in list.iter().enumerate() {
                if i > 0 {
                    print!(",");
                }
                display_value(value);
            }
            print!("]");
        }
        Value::Dict(dict) => {
            print!("{{");
            for (i, (key, value)) in dict.iter().enumerate() {
                if i > 0 { print!(",");
                    }
                print!("\"{}\":", String::from_utf8_lossy(key));
                display_value(value);
            }
            print!("}}");
        }, 
    }
}
