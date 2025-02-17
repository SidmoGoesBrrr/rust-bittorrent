use serde_json;

// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded: &str) -> (serde_json::Value, usize) {
    let first_char = encoded.chars().next().unwrap();

    if first_char.is_ascii_digit() {
        // (1) Parse string: "<length>:<contents>"
        let colon_index = encoded.find(':').unwrap();
        let length_str = &encoded[..colon_index];
        let length: usize = length_str.parse().unwrap();

        let start_of_str = colon_index + 1;
        let end_of_str   = start_of_str + length;
        let actual_str   = &encoded[start_of_str..end_of_str];

        let consumed = end_of_str; // how many chars were consumed from `encoded`

        (serde_json::Value::String(actual_str.to_owned()), consumed)
    } else if first_char == 'i' {
        // (2) Parse integer: "i<digits>e"
        let end_index = encoded.find('e').unwrap();
        let number_str = &encoded[1..end_index];
        let number: i64 = number_str.parse().unwrap();

        // We consumed from index 0 to `end_index` inclusive => end_index + 1
        let consumed = end_index + 1;

        (serde_json::Value::Number(number.into()), consumed)
    } else if first_char == 'l' {
        // (3) Parse list: "l<items>e"
        let mut list = Vec::new();
        // Start after 'l'
        let mut index = 1;

        // Decode elements until we hit 'e'
        while encoded.chars().nth(index).unwrap() != 'e' {
            let (value, used) = decode_bencoded_value(&encoded[index..]);
            list.push(value);
            index += used; // advance by how many chars were used
        }

        // Skip the 'e' at the end
        index += 1;
        (serde_json::Value::Array(list), index)
    } else if first_char == 'd' {
        // (4) Parse dictionary: "d<key><value><key><value>...e"
        let mut map = serde_json::Map::new();
        // Start after 'd'
        let mut index = 1;

        // Decode pairs until we see 'e'
        while encoded.chars().nth(index).unwrap() != 'e' {
            // Decode key
            let (key_val, used_key) = decode_bencoded_value(&encoded[index..]);
            index += used_key;

            // Decode value
            let (val_val, used_val) = decode_bencoded_value(&encoded[index..]);
            index += used_val;

            // Dictionary keys in Bencode are always strings. Enforce that here:
            let key_str = key_val.as_str()
                .expect("Bencode dictionary key wasn't a string!")
                .to_owned();

            map.insert(key_str, val_val);
        }

        // Skip the 'e' that ends the dictionary
        index += 1;

        (serde_json::Value::Object(map), index)
    } else {
        panic!("Unhandled or invalid bencoded value: {}", encoded);
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