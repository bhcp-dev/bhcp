use std::fmt::Write;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Text(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(Vec<(String, Value)>),
    Tag(u64, Box<Value>),
}

impl Value {
    pub fn map<const N: usize>(entries: [(&str, Value); N]) -> Self {
        Self::owned_map(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    pub fn owned_map(mut entries: Vec<(String, Value)>) -> Self {
        entries.sort_by(|left, right| {
            encoded_text_len(&left.0)
                .cmp(&encoded_text_len(&right.0))
                .then_with(|| left.0.as_bytes().cmp(right.0.as_bytes()))
        });
        Self::Map(entries)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        let Value::Map(entries) = self else {
            return None;
        };
        entries
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value))
    }

    pub fn kind(&self) -> Option<&str> {
        match self.get("kind") {
            Some(Value::Text(kind)) => Some(kind),
            _ => None,
        }
    }

    pub fn to_json_pretty(&self) -> String {
        let mut output = String::new();
        self.write_json(&mut output, 0);
        output
    }

    fn write_json(&self, output: &mut String, indent: usize) {
        match self {
            Self::Null => output.push_str("null"),
            Self::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Self::Integer(value) => write!(output, "{value}").unwrap(),
            Self::Text(value) => write_json_string(output, value),
            Self::Bytes(bytes) => {
                output.push('"');
                for byte in bytes {
                    write!(output, "{byte:02x}").unwrap();
                }
                output.push('"');
            }
            Self::Tag(tag, value) => {
                output.push_str("{\n");
                output.push_str(&" ".repeat(indent + 2));
                write_json_string(output, "$tag");
                writeln!(output, ": {tag},").unwrap();
                output.push_str(&" ".repeat(indent + 2));
                write_json_string(output, "value");
                output.push_str(": ");
                value.write_json(output, indent + 2);
                output.push('\n');
                output.push_str(&" ".repeat(indent));
                output.push('}');
            }
            Self::Array(items) => {
                if items.is_empty() {
                    output.push_str("[]");
                    return;
                }
                output.push_str("[\n");
                for (index, item) in items.iter().enumerate() {
                    output.push_str(&" ".repeat(indent + 2));
                    item.write_json(output, indent + 2);
                    if index + 1 != items.len() {
                        output.push(',');
                    }
                    output.push('\n');
                }
                output.push_str(&" ".repeat(indent));
                output.push(']');
            }
            Self::Map(entries) => {
                if entries.is_empty() {
                    output.push_str("{}");
                    return;
                }
                output.push_str("{\n");
                for (index, (key, value)) in entries.iter().enumerate() {
                    output.push_str(&" ".repeat(indent + 2));
                    write_json_string(output, key);
                    output.push_str(": ");
                    value.write_json(output, indent + 2);
                    if index + 1 != entries.len() {
                        output.push(',');
                    }
                    output.push('\n');
                }
                output.push_str(&" ".repeat(indent));
                output.push('}');
            }
        }
    }
}

fn encoded_text_len(value: &str) -> usize {
    let length = value.len();
    length
        + if length < 24 {
            1
        } else if length <= u8::MAX as usize {
            2
        } else if length <= u16::MAX as usize {
            3
        } else {
            5
        }
}

fn write_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                write!(output, "\\u{:04x}", character as u32).unwrap()
            }
            character => output.push(character),
        }
    }
    output.push('"');
}
