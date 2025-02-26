use std::collections::HashMap;

fn decode_query(encoded: &str) -> Option<String> {
    let mut decoded = String::new();
    let mut bytes = encoded.bytes();

    while let Some(byte) = bytes.next() {
        match byte {
            b'%' => {
                let first = bytes.next()?;
                let second = bytes.next()?;
                let hex =
                    u8::from_str_radix(&format!("{}{}", first as char, second as char), 16).ok()?;
                decoded.push(hex as char)
            }

            b'+' => decoded.push(' ' as char),

            byte => decoded.push(byte as char),
        }
    }

    Some(decoded)
}

pub fn map(encoded: &str) -> Option<HashMap<String, String>> {
    let mut map = HashMap::new();

    let Some(query) = decode_query(encoded) else {
        return None;
    };

    let pairs = query.split("&");

    for pair in pairs {
        if pair.is_empty() {
            continue;
        }

        let mut pair = pair.split("=");

        let Some(key) = pair.next() else {
            break;
        };

        let value = pair.next().unwrap_or("");

        map.insert(key.to_string(), value.to_string());
    }

    Some(map)
}
