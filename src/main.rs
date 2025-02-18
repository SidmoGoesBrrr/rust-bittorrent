use serde::{Deserialize, Serialize};
use serde_bencode::{de, ser};
use serde_json;
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::{Read, Result};

/// Structure representing the Torrent metainfo file
#[derive(Debug, Deserialize)]
struct Torrent {
    announce: String,
    info: Info,
}

/// Structure for the "info" dictionary
#[derive(Debug, Deserialize, Serialize)]
struct Info {
    length: u64,
    name: String,
    #[serde(rename = "piece length")]
    piece_length: u64,
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>, // Treat as raw bytes
}

fn calculate_info_hash(info: &Info) -> String {
    // Bencode the info dictionary back into bytes
    let bencoded_info = match ser::to_bytes(info) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to Bencode 'info' dictionary: {:?}", e);
            std::process::exit(1);
        }
    };

    // Compute SHA-1 hash
    let mut hasher = Sha1::new();
    hasher.update(bencoded_info);
    let result = hasher.finalize();

    // Convert hash to hex string
    result.iter().map(|byte| format!("{:02x}", byte)).collect()
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

/// Function to extract and format SHA-1 piece hashes
fn extract_piece_hashes(pieces: &[u8]) -> Vec<String> {
    const SHA1_HASH_SIZE: usize = 20;
    let mut hashes = Vec::new();

    // Ensure pieces are properly sized
    if pieces.len() % SHA1_HASH_SIZE != 0 {
        eprintln!("Invalid pieces length: not a multiple of 20 bytes");
        std::process::exit(1);
    }

    for chunk in pieces.chunks(SHA1_HASH_SIZE) {
        let hash_hex: String = chunk.iter().map(|byte| format!("{:02x}", byte)).collect();
        hashes.push(hash_hex);
    }

    hashes
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
        let info_hash = calculate_info_hash(&torrent.info);
        let piece_hashes = extract_piece_hashes(&torrent.info.pieces);

        let _pieces_hash: Vec<String> = Vec::new();
        // Print required information
        println!("Tracker URL: {}", torrent.announce);
        println!("Length: {}", torrent.info.length);
        println!("Info Hash: {}", info_hash);
        println!("Piece Length: {}", torrent.info.piece_length);
        println!("Piece Hashes:");
        for hash in piece_hashes {
            println!("{}", hash);
        }
        
    } else {
        eprintln!("Unknown command: {}", command);
        std::process::exit(1);
    }

    Ok(())
}
