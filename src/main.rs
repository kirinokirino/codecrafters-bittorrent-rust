use serde::{Deserialize, Serialize};
use serde_bencode::{ser, de, value::Value};
use serde_bytes::ByteBuf;
use sha1::{Sha1, Digest};

use std::env;
use std::fs::read;

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value: Value = de::from_str(encoded_value).unwrap();
        println!("{}", displayed_value(decoded_value));
    } else if command == "info" {
        let torrent_path = &args[2];
        let torrent = read(torrent_path).unwrap();
        let decoded_value: Torrent = de::from_bytes::<Torrent>(&torrent).unwrap();
        let announce = decoded_value.announce;
        let length = decoded_value.info.length;
        let mut hasher = Sha1::new();
        hasher.update(ser::to_bytes(&decoded_value.info).unwrap());
        let info_hash = hasher.finalize();
        println!("Tracker URL: {}\nLength: {}\nInfo Hash: {:x}", announce, length, info_hash);
    } else {
        println!("unknown command: {}", args[1])
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Torrent {
    announce: String,
    #[serde(rename = "created by")]
    created_by: String,
    info: Info, //HashMap<Vec<u8>, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Info {
    length: i64,
    name: String,
    #[serde(rename = "piece length")]
    piece_length: i64,
    pieces: ByteBuf,
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
            let mut displayed =
                array
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
        assert_eq!(
            "[\"hello\",42]",
            displayed_value(de::from_str(list).unwrap())
        );

        let dict = "d3:foo3:bar5:helloi52ee";
        assert_eq!(
            r#"{"foo":"bar","hello":52}"#,
            displayed_value(de::from_str(dict).unwrap())
        );

        let empty_dict = "de";
        assert_eq!(r#"{}"#, displayed_value(de::from_str(empty_dict).unwrap()));
    }
}
