use reqwest;
use serde::{Deserialize, Serialize};
use serde_bencode::{de, ser, value::Value};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};

use std::env;
use std::fs::read;

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    if command == "decode" {
        println!("{}", decode(&args[2]));
    } else if command == "info" {
        println!("{}", torrent_info(&args[2]));
    } else {
        println!("unknown command: {}", args[1])
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Torrent {
    announce: String,
    #[serde(rename = "created by", default)]
    created_by: Option<String>,
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

fn parse_torrent_file(path: &str) -> Torrent {
    let contents = read(path).unwrap();
    de::from_bytes::<Torrent>(&contents).unwrap()
}

fn torrent_info(path: &str) -> String {
    let torrent = parse_torrent_file(path);
    let announce = torrent.announce;
    let length = torrent.info.length;
    let mut hasher = Sha1::new();
    hasher.update(ser::to_bytes(&torrent.info).unwrap());
    let info_hash = hasher.finalize();
    let piece_length = torrent.info.piece_length;
    let piece_hashes = torrent.info.pieces.as_slice();
    let mut output = format!(
        "Tracker URL: {}\nLength: {}\nInfo Hash: {:x}\nPiece Length: {}\nPiece Hashes:",
        announce, length, info_hash, piece_length
    );
    for (i, byte) in piece_hashes.iter().enumerate() {
        if i % 20 == 19 {
            output.push_str(&format!("{byte:02x}\n"));
        } else {
            output.push_str(&format!("{byte:02x}"));
        }
    }
    output.trim_end_matches('\n').to_string()
}

fn decode(value: &str) -> String {
    let decoded_value: Value = de::from_str(value).unwrap();
    format!("{}", displayed_value(decoded_value))
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
    fn test_torrent_info() {
        assert_eq!(
            r#"Tracker URL: http://bittorrent-test-tracker.codecrafters.io/announce
Length: 92063
Info Hash: d69f91e6b2ae4c542468d1073a71d4ea13879a7f
Piece Length: 32768
Piece Hashes:e876f67a2a8886e8f36b136726c30fa29703022d
6e2275e604a0766656736e81ff10b55204ad8d35
f00d937a0213df1982bc8d097227ad9e909acc17"#,
            torrent_info("sample.torrent")
        );
    }

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
