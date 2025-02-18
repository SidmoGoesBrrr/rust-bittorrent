use serde::Deserialize;
use serde_bencode::de;
use serde_json;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Result};

/// Structure representing the Torrent metainfo file
#[derive(Debug, Deserialize)]
struct Torrent {
    announce: String,
    info: Info,
}

/// Structure for the "info" dictionary
#[derive(Debug, Deserialize)]
struct Info {
    length: u64,
    name: String,
    #[serde(rename = "piece length")]
    piece_length: u64,
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>, // Treat as raw bytes
}

// A helper to safely fetch a char from a &str
fn safe_char_at(s: &str, char_index: usize) -> char {
    // s.chars().nth(char_index) returns an Option<char>. If None, we panic with a custom message.
    s.chars().nth(char_index).unwrap_or_else(|| {
        panic!(
            "\n\
Ran out of characters at char index={} (string length in chars={}).\n\
This usually happens because the .torrent file has invalid UTF-8 bytes\n\
which got replaced by one or more '�' characters.\n\
If the Bencode has large binary data (e.g. the 'pieces' field), you may hit partial chars.\n\
To handle this correctly, consider a byte-based parser.\n",
            char_index,
            s.chars().count()
        )
    })
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded: &str) -> (serde_json::Value, usize) {
    // Safely grab the first char
    let first_char = match encoded.chars().next() {
        Some(c) => c,
        None => panic!("Empty string while expecting bencoded data"),
    };

    if first_char.is_ascii_digit() {
        // 1) Parse string: "<length>:<contents>"
        let colon_index = encoded.find(':').unwrap();
        let length_str = &encoded[..colon_index];
        let length: usize = length_str.parse().unwrap();

        let start_of_str = colon_index + 1;
        let end_of_str = start_of_str + length;
        if end_of_str > encoded.len() {
            panic!(
                "String length {} extends beyond encoded data length {}",
                length,
                encoded.len()
            );
        }

        let actual_str = &encoded[start_of_str..end_of_str];
        let consumed = end_of_str;

        (serde_json::Value::String(actual_str.to_owned()), consumed)
    } else if first_char == 'i' {
        // 2) Parse integer: "i<digits>e"
        let end_index = encoded.find('e').unwrap();
        let number_str = &encoded[1..end_index];
        let number: i64 = number_str.parse().unwrap();

        let consumed = end_index + 1;
        (serde_json::Value::Number(number.into()), consumed)
    } else if first_char == 'l' {
        // 3) Parse list: "l<items>e"
        let mut list = Vec::new();
        let mut index = 1; // after 'l'

        // Use safe_char_at here instead of .nth(index).unwrap()
        while safe_char_at(encoded, index) != 'e' {
            // parse the next element from the substring
            let (value, used) = decode_bencoded_value(&encoded[index..]);
            list.push(value);
            index += used;
            // If index goes out of range, safe_char_at() will panic with a friendlier message
        }

        index += 1; // skip 'e'
        (serde_json::Value::Array(list), index)
    } else if first_char == 'd' {
        // 4) Parse dictionary: "d<key><value>...e"
        let mut map = serde_json::Map::new();
        let mut index = 1; // after 'd'

        while safe_char_at(encoded, index) != 'e' {
            // parse key
            let (key_val, used_key) = decode_bencoded_value(&encoded[index..]);
            index += used_key;

            // parse value
            let (val_val, used_val) = decode_bencoded_value(&encoded[index..]);
            index += used_val;

            // dictionary keys must be strings
            let key_str = key_val
                .as_str()
                .expect("Bencode dictionary key wasn't a string!")
                .to_owned();

            map.insert(key_str, val_val);
        }

        index += 1; // skip 'e'
        (serde_json::Value::Object(map), index)
    } else {
        panic!("Unhandled or invalid bencoded value: {}", encoded);
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let command = &args[1];
    let file_path: &String = &args[2];

    if command == "decode" {
        let encoded_value = &args[2];
        let (decoded_value, _used) = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value);
    } else if command == "info" {
        let file_path: &String = &args[2];
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // Deserialize the Bencoded data directly from bytes
        let torrent: Torrent = match de::from_bytes(&buffer) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to parse .torrent file: {:?}", e);
                std::process::exit(1);
            }
        };

        // Print required information
        println!("Tracker URL: {}", torrent.announce);
        println!("Length: {}", torrent.info.length);
    } else {
        eprintln!("Unknown command: {}", command);
        std::process::exit(1);
    }

    Ok(())
}
