use serde_bencode::{de, value::Value};

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value: Value = de::from_str(encoded_value).unwrap();
        println!("{}", displayed_value(decoded_value));
    } else {
        println!("unknown command: {}", args[1])
    }
}

fn displayed_value(value: Value) -> String {
    match value {
        Value::Bytes(bytes) => {
            format!("\"{}\"", String::from_utf8_lossy(&bytes))
        }
        Value::Int(int) => format!("{int}"),
        Value::List(list) => format!(
            "[{}]",
            list.into_iter().fold(String::new(), |mut acc, value| {
                if !acc.is_empty() {
                    acc.push(',');
                }
                format!("{acc}{}", displayed_value(value))
            })
        ),
        Value::Dict(dict) => {
            let mut array: Vec<(String, String)> = dict
                .into_iter()
                .map(|(key, value)| {
                    (
                        format!("\"{}\"", String::from_utf8_lossy(&key)),
                        displayed_value(value),
                    )
                })
                .collect();
            if array.is_empty() {
                return "{}".to_string();
            }
            array.sort_by(|(key1, _), (key2, _)| std::cmp::Ord::cmp(key1, key2));
            let mut displayed = array
                .into_iter()
                .fold(String::new(), |mut acc, (next_key, next_value)| {
                    if acc.is_empty() {
                        acc.push('{');
                    } else {
                        acc.push(',');
                    }
                    acc = format!("{acc}{next_key}:{next_value}");
                    acc
                });
            displayed.push('}');
            displayed
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn test_display_bencode_values() {
        let bytes = "5:hello";
        assert_eq!("\"hello\"", displayed_value(de::from_str(bytes).unwrap()));
        
        let number = "i42e";
        assert_eq!("42", displayed_value(de::from_str(number).unwrap()));
        
        let list = "l5:helloi42ee";
        assert_eq!("[\"hello\",42]", displayed_value(de::from_str(list).unwrap()));
        
        let dict = "d3:foo3:bar5:helloi52ee";
        assert_eq!(r#"{"foo":"bar","hello":52}"#, displayed_value(de::from_str(dict).unwrap()));
        
        let empty_dict = "de";
        assert_eq!(r#"{}"#, displayed_value(de::from_str(empty_dict).unwrap()));
    }
}
