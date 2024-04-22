use alloc::vec::Vec;

pub enum TomlValue<'a> {
    String(&'a str),
    Integer(i64),
}

pub fn parse_toml(toml_data: &str) -> Option<Vec<(&str, TomlValue)>> {
    let mut result = Vec::new();

    let mut current_key = "";
    let mut current_value = "";

    for line in toml_data.lines() {
        let trimmed_line = line.trim();

        if trimmed_line.starts_with('#') || trimmed_line.is_empty() {
            continue;
        }

        if let Some((key, value)) = extract_key_value(trimmed_line) {
            if !current_key.is_empty() {
                insert_into_result(&mut result, current_key, current_value);
            }
            current_key = key;
            current_value = value;
        } else {
            current_value = trimmed_line;
        }
    }

    if !current_key.is_empty() {
        insert_into_result(&mut result, current_key, current_value);
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}
fn extract_key_value(line: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = line.split('=').map(|s| s.trim()).collect();

    if parts.len() == 2 {
        let key = parts[0];
        let mut value = parts[1];
        if value.starts_with('"') && value.ends_with('"') {
            value = &value[1..value.len() - 1]; // Remove leading and trailing quotes
        }
        Some((key, value))
    } else {
        None
    }
}

fn insert_into_result<'a>(
    result: &mut Vec<(&'a str, TomlValue<'a>)>,
    key: &'a str,
    value: &'a str,
) {
    if let Ok(integer) = value.parse::<i64>() {
        result.push((key, TomlValue::Integer(integer)));
    } else {
        result.push((key, TomlValue::String(value)));
    }
}
