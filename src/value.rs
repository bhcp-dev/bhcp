#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i128),
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
