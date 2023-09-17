use serde_json;
use std::env;

// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    // If encoded_value starts with a digit, it's a number
    if encoded_value.chars().next().unwrap().is_digit(10) {
        // Example: "5:hello" -> "5"
        let colon_index = encoded_value.find(':').unwrap();
        let number_string = &encoded_value[..colon_index];
        let number = number_string.parse::<i64>().unwrap();
        let string_start = colon_index + 1;
        let string = &encoded_value[string_start..string_start + number as usize];
        return serde_json::Value::String(string.to_string());
    } else if encoded_value.chars().next().unwrap() == 'i' {
        // Example: "i42e" -> "42"
        let end_index = encoded_value.find('e').unwrap();
        let number_string = &encoded_value[..end_index];
        return serde_json::Number::from_f64(ascii_to_number(number_string) as f64)
            .unwrap()
            .into();
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

fn ascii_to_number(s: &str) -> i64 {
    let negative = s.starts_with('-');
    let minus_index = if negative { 1 } else { 0 };
    let num_string = &s[minus_index..s.len()];
    let mut sum = 0;
    for (i, ch) in num_string.chars().enumerate() {
        sum += (ch.to_digit(10).unwrap() as i64)
            * i64::from(10).pow(num_string.len() as u32 - (i as u32 + 1));
    }
    return if negative { -sum } else { sum };
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        // println!("Logs from your program will appear here!");

        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}

#[cfg(test)]
mod tests {
    use crate::ascii_to_number;

    #[test]
    fn ascii_to_num_test() {
        assert_eq!(1, ascii_to_number("1"));
        assert_eq!(12, ascii_to_number("12"));
        assert_eq!(123, ascii_to_number("123"));
        assert_eq!(1234, ascii_to_number("1234"));
        assert_eq!(12345, ascii_to_number("12345"));
        assert_eq!(-10, ascii_to_number("-10"));
        assert_eq!(-10000, ascii_to_number("-10000"));
    }
}
