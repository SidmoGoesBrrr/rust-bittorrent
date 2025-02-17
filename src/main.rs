use serde_json;
use std::env;

// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded: &str) -> (serde_json::Value, usize) {
    let first_char = encoded.chars().next().unwrap();

    if first_char.is_ascii_digit() {
        // 1. Parse string: "<length>:<contents>"
        // Find colon:
        let colon_index = encoded.find(':').unwrap();
        let length_str = &encoded[..colon_index];
        let length: usize = length_str.parse().unwrap();
        
        // The bencoded string is colon_index+1 + length
        // e.g. "4:pear" has 1+4=5 bytes after the colon
        let start_of_str = colon_index + 1;
        let end_of_str   = start_of_str + length; 
        let actual_str = &encoded[start_of_str..end_of_str];
        
        // We consumed colon_index+1+length total characters
        let consumed = end_of_str;
        
        // Return the JSON string and how many bytes from the bencoded input we consumed
        (
            serde_json::Value::String(actual_str.to_owned()),
            consumed
        )
    }
    else if first_char == 'i' {
        // 2. Parse integer: "i<digits>e"
        // Find the terminating 'e'
        let end_index = encoded.find('e').unwrap();
        // number is between 'i' and 'e'
        let number_str = &encoded[1..end_index];
        let number: i64 = number_str.parse().unwrap();
        
        // We consumed from index 0 to end_index (inclusive) => end_index + 1
        let consumed = end_index + 1;
        
        (
            serde_json::Value::Number(number.into()),
            consumed
        )
    }
    else if first_char == 'l' {
        // 3. Parse list: "l<items>e"
        let mut list = Vec::new();
        
        // We start after 'l', so we've consumed 1 char so far
        let mut index = 1;
        
        // While we haven't hit 'e' (the end of the list):
        while encoded.chars().nth(index).unwrap() != 'e' {
            let (value, used) = decode_bencoded_value(&encoded[index..]);
            list.push(value);
            
            // Advance by the amount used in the substring
            index += used;
        }
        
        // Skip the 'e' that ends the list
        index += 1;
        
        (serde_json::Value::Array(list), index)
    }
    else {
        panic!("Unhandled or invalid bencoded value: {encoded}")
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        
        // decode returns (value, used_bytes), but we only need the value here
        let (decoded_value, _used) = decode_bencoded_value(encoded_value);
        
        println!("{}", decoded_value);
    } else {
        println!("unknown command: {}", command);
    }
}