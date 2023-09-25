use reqwest;
use serde::{Deserialize, Serialize};
use serde_bencode::{de, ser, value::Value};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};
use tempfile;

use std::convert::TryFrom;
use std::env;
use std::fs::{read, File};
use std::io::prelude::*;
use std::net::TcpStream;

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    if command == "decode" {
        println!("{}", decode(&args[2]));
    } else if command == "info" {
        println!("{}", torrent_info(&args[2]));
    } else if command == "peers" {
        let peers = peers_for_torrent(&args[2]);
        peers.iter().for_each(|peer| println!("{peer}"));
    } else if command == "handshake" {
        let mut stream = TcpStream::connect(&args[3]).unwrap();
        let handshake_peer = handshake(&mut stream, &args[2]);
        println!("Peer ID: {handshake_peer}");
    } else if command == "download_piece" {
        let _dash_o = &args[2];
        let download_path = &args[3];
        let torrent_path = &args[4];
        let piece_idx = &args[5];

        //let temp_dir = tempfile::tempdir().unwrap();
        //let (_tmp_folder, file_path) = download_path.split_once("/tmp/").unwrap();
        let temp_file_path = download_path; //temp_dir.path().join(file_path);
        dbg!(&temp_file_path);
        let mut file = File::create(temp_file_path).unwrap();

        let torrent = parse_torrent_file(torrent_path);
        let peers = peers_for_torrent(torrent_path);
        let mut stream = TcpStream::connect(peers.first().unwrap()).unwrap();
        handshake(&mut stream, torrent_path);

        // bitfield message <-
        let (message_type, _) = recieve_message(&mut stream);
        assert_eq!(message_type, PeerMessage::Bitfield);

        // interested message ->
        let bytes_sent = send_message(&mut stream, PeerMessage::Interested, None);
        assert!(bytes_sent > 0);

        // unchoke message <-
        let (message_type, _) = recieve_message(&mut stream);
        assert_eq!(message_type, PeerMessage::Unchoke);

        // request message ->
        let file_size = torrent.info.length as u32;
        let piece_size = torrent.info.piece_length as u32;
        let last_piece_size = file_size % piece_size;
        let full_pieces = file_size / piece_size;

        let piece_index = piece_idx.parse::<u32>().unwrap();
        let current_piece_size = if (piece_index as u32) < full_pieces {
            piece_size
        } else if (piece_index as u32) == full_pieces {
            last_piece_size
        } else {
            panic!("piece index out of bounds!");
        };
        let mut piece_data = Vec::with_capacity(current_piece_size as usize);

        // loop for blocks of the piece

        let mut block_offset = 0u32;
        while block_offset < current_piece_size {
            let block_size = (current_piece_size - block_offset).min(16 * 1024);

            let message_payload = [
                piece_index.to_be_bytes(),
                block_offset.to_be_bytes(),
                block_size.to_be_bytes(),
            ]
            .concat();
            let bytes_sent = send_message(&mut stream, PeerMessage::Request, Some(message_payload));
            assert!(bytes_sent >= 4 * 3);

            // piece message <-
            let (message_type, payload) = recieve_message(&mut stream);
            assert_eq!(message_type, PeerMessage::Piece);
            assert!(payload.is_some());
            if let Some(payload) = payload {
                assert!(payload.len() >= 4 + 4);
                let recieved_piece_index = u32::from_be_bytes(payload[0..4].try_into().unwrap());
                assert_eq!(recieved_piece_index, piece_index);
                let offset_within_the_piece = u32::from_be_bytes(payload[4..8].try_into().unwrap());
                assert_eq!(offset_within_the_piece, block_offset);
                let recieved_data = &payload[8..];
                block_offset += recieved_data.len() as u32;
                piece_data.extend_from_slice(recieved_data);
            }
        }
        let piece_hash: Vec<u8> = torrent.info.pieces.into_iter().take(20).collect();
        if hash(&piece_data) != piece_hash {
            eprintln!("ERROR: hashes are different");
        }
        file.write_all(piece_data.as_slice()).unwrap();
    } else {
        println!("unknown command: {}", args[1])
    }
}

fn recieve_message(stream: &mut TcpStream) -> (PeerMessage, Option<Vec<u8>>) {
    let mut length = [0u8; 4];
    let _ = stream.read_exact(&mut length[..]);
    let length = u32::from_be_bytes(length);

    let mut message = [0u8].repeat(length as usize);
    stream.read_exact(&mut message).unwrap();
    let message_type: PeerMessage = message[0].try_into().unwrap();
    dbg!(&message_type, &length);
    (message_type, Some(message[1..].to_vec()))
}

fn send_message(
    stream: &mut TcpStream,
    message_type: PeerMessage,
    payload: Option<Vec<u8>>,
) -> usize {
    let message = if let Some(payload) = payload {
        let length = payload.len() + 1;
        [
            length.to_be_bytes().as_slice(),
            [message_type as u8].as_slice(),
            payload.as_slice(),
        ]
        .concat()
    } else {
        [
            1u32.to_be_bytes().as_slice(),
            [message_type as u8].as_slice(),
        ]
        .concat()
    };
    stream.write(&message).unwrap()
}

#[derive(Debug, PartialEq, Eq)]
enum PeerMessage {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

impl TryFrom<u8> for PeerMessage {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 8 {
            Err(())
        } else {
            Ok(unsafe { std::mem::transmute(value) })
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TrackerResponse {
    interval: i64,
    peers: ByteBuf,
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

fn handshake(stream: &mut TcpStream, torrent_path: &str) -> String {
    let torrent = parse_torrent_file(torrent_path);
    //let separator_index = peer_to_handshake.find(':').unwrap();
    //let (ip, port) = peer_to_handshake.split_at(separator_index);
    let protocol_name = "BitTorrent protocol";
    let protocol_name_length = protocol_name.chars().count() as u8;
    let reserved = [0u8; 8];
    let info_hash = info_hash(&torrent.info);
    let peer_id = "00112233445566778899";
    let handshake_message = [
        &[protocol_name_length],
        protocol_name.as_bytes(),
        &reserved,
        info_hash.as_slice(),
        peer_id.as_bytes(),
    ]
    .concat();
    let _sent = stream.write(&handshake_message).unwrap();
    let mut response_buffer: [u8; 68] = [0u8; 68];
    let _received = stream.read(&mut response_buffer).unwrap();
    response_buffer
        .iter()
        .skip(68 - 20)
        .map(|byte| format!("{:02x}", byte))
        .collect()
}

fn peers_for_torrent(path: &str) -> Vec<String> {
    let torrent = parse_torrent_file(path);
    let info_hash: String = formatted_info_hash(&torrent.info, "%");
    let left = torrent.info.length;
    let address = torrent.announce;
    let response = request_to_tracker(left, &address, &info_hash);
    parse_peers(&response.peers)
}

fn request_to_tracker(left: i64, address: &str, info_hash: &str) -> TrackerResponse {
    let peer_id = "00112233445566778899";
    let port = 6881;
    let uploaded = 0;
    let downloaded = 0;
    let compact = 1;
    let body = reqwest::blocking::get(format!("{address}?info_hash={info_hash}&peer_id={peer_id}&port={port}&uploaded={uploaded}&downloaded={downloaded}&left={left}&compact={compact}"));
    let response_text = body.unwrap().bytes().unwrap();
    let response: Result<TrackerResponse, serde_bencode::error::Error> =
        de::from_bytes(&response_text);
    if response.is_err() {
        dbg!(&response_text, response.err().unwrap());
        panic!();
    }
    response.unwrap()
}

fn parse_peers(bytes: &[u8]) -> Vec<String> {
    let mut peers: Vec<String> = Vec::new();
    bytes.chunks(6).for_each(|chunk| {
        let ip = &chunk[0..4];
        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
        let peer = format!("{}.{}.{}.{}:{port}", ip[0], ip[1], ip[2], ip[3]);
        peers.push(peer);
    });
    peers
}

fn parse_torrent_file(path: &str) -> Torrent {
    let contents = read(path).unwrap();
    de::from_bytes::<Torrent>(&contents).unwrap()
}

fn torrent_info(path: &str) -> String {
    let torrent = parse_torrent_file(path);
    let announce = torrent.announce;
    let length = torrent.info.length;
    let piece_length = torrent.info.piece_length;
    let piece_hashes = torrent.info.pieces.as_slice();
    let mut output = format!(
        "Tracker URL: {}\nLength: {}\nInfo Hash: {}\nPiece Length: {}\nPiece Hashes:",
        announce,
        length,
        formatted_info_hash(&torrent.info, ""),
        piece_length
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

fn info_hash(info: &Info) -> Vec<u8> {
    hash(ser::to_bytes(info).unwrap().as_slice())
}

fn hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

fn formatted_info_hash(info: &Info, separator: &str) -> String {
    info_hash(info)
        .iter()
        .map(|byte| format!("{separator}{byte:02x}"))
        .collect()
}

fn decode(value: &str) -> String {
    let decoded_value: Value = de::from_str(value).unwrap();
    displayed_value(decoded_value).to_string()
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
    fn test_handshake() {
        let connect_to = "178.62.82.89:51470";
        let mut stream = TcpStream::connect(connect_to).unwrap();
        let hardcoded_peer = "2d524e302e302e302d2e99080f6fd8278cf6e0f2";
        let handshake_peer = handshake(&mut stream, "sample.torrent");
        assert_eq!(hardcoded_peer, handshake_peer);
    }

    #[test]
    fn test_peers() {
        let peers = peers_for_torrent("sample.torrent");
        let hardcoded_peers = vec![
            "178.62.82.89:51470",
            "165.232.33.77:51467",
            "178.62.85.20:51489",
        ];
        for (peer1, peer2) in peers.iter().zip(hardcoded_peers.iter()) {
            assert_eq!(peer1, peer2);
        }
    }

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
