use std::collections::HashSet;

use crate::diagnostic::{Diagnostic, Result};
use crate::value::Value;

pub fn encode_deterministic(value: &Value) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    encode(value, &mut output)?;
    Ok(output)
}

fn encode(value: &Value, output: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Null => output.push(0xf6),
        Value::Bool(false) => output.push(0xf4),
        Value::Bool(true) => output.push(0xf5),
        Value::Integer(value) if *value >= 0 && *value <= i128::from(u64::MAX) => {
            write_head(0, *value as u64, output)
        }
        Value::Integer(value) if *value < 0 && *value >= -1 - i128::from(u64::MAX) => {
            write_head(1, (-1 - *value) as u64, output)
        }
        Value::Integer(_) => {
            return Err(Diagnostic::plain(
                "BHCP3005",
                "integer exceeds the canonical CBOR int domain",
            ));
        }
        Value::Text(value) => {
            validate_text(value)?;
            write_head(3, value.len() as u64, output);
            output.extend_from_slice(value.as_bytes());
        }
        Value::Bytes(bytes) => {
            write_head(2, bytes.len() as u64, output);
            output.extend_from_slice(bytes);
        }
        Value::Array(items) => {
            write_head(4, items.len() as u64, output);
            for item in items {
                encode(item, output)?;
            }
        }
        Value::Map(entries) => {
            let mut encoded = Vec::with_capacity(entries.len());
            let mut keys = HashSet::new();
            for (key, value) in entries {
                if !keys.insert(key) {
                    return Err(Diagnostic::plain(
                        "BHCP3004",
                        format!("duplicate map key {key:?}"),
                    ));
                }
                let mut encoded_key = Vec::new();
                encode(&Value::Text(key.clone()), &mut encoded_key)?;
                let mut encoded_value = Vec::new();
                encode(value, &mut encoded_value)?;
                encoded.push((encoded_key, encoded_value));
            }
            encoded.sort_by(|left, right| {
                left.0
                    .len()
                    .cmp(&right.0.len())
                    .then_with(|| left.0.cmp(&right.0))
            });
            write_head(5, encoded.len() as u64, output);
            for (key, value) in encoded {
                output.extend(key);
                output.extend(value);
            }
        }
        Value::Tag(tag, value) => {
            write_head(6, *tag, output);
            encode(value, output)?;
        }
    }
    Ok(())
}

fn validate_text(value: &str) -> Result<()> {
    if value
        .chars()
        .all(|character| character.is_ascii() || character == '§')
    {
        return Ok(());
    }
    Err(Diagnostic::plain(
        "BHCP3002",
        "this dependency-free canonical slice accepts ASCII text plus the precomposed § sigil; other Unicode is explicitly unsupported",
    ))
}

fn write_head(major: u8, value: u64, output: &mut Vec<u8>) {
    match value {
        0..=23 => output.push((major << 5) | value as u8),
        24..=0xff => output.extend_from_slice(&[(major << 5) | 24, value as u8]),
        0x100..=0xffff => {
            output.push((major << 5) | 25);
            output.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            output.push((major << 5) | 26);
            output.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            output.push((major << 5) | 27);
            output.extend_from_slice(&value.to_be_bytes());
        }
    }
}

pub fn decode_deterministic(bytes: &[u8]) -> Result<Value> {
    let mut decoder = Decoder { bytes, cursor: 0 };
    let value = decoder.value()?;
    if decoder.cursor != bytes.len() {
        return Err(Diagnostic::plain(
            "BHCP3005",
            "trailing bytes after CBOR item",
        ));
    }
    if encode_deterministic(&value)? != bytes {
        return Err(Diagnostic::plain(
            "BHCP3005",
            "CBOR item is not in deterministic form",
        ));
    }
    Ok(value)
}

struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl Decoder<'_> {
    fn value(&mut self) -> Result<Value> {
        let initial = self.byte()?;
        let major = initial >> 5;
        let additional = initial & 31;
        if additional == 31 {
            return Err(Diagnostic::plain(
                "BHCP3005",
                "indefinite-length CBOR is forbidden",
            ));
        }
        match major {
            0 => Ok(Value::Integer(i128::from(self.argument(additional)?))),
            1 => {
                let value = self.argument(additional)?;
                let signed = -1i128 - i128::from(value);
                Ok(Value::Integer(signed))
            }
            2 => {
                let length = self.length(additional)?;
                Ok(Value::Bytes(self.take(length)?.to_vec()))
            }
            3 => {
                let length = self.length(additional)?;
                let text = std::str::from_utf8(self.take(length)?)
                    .map_err(|_| Diagnostic::plain("BHCP3005", "CBOR text is not UTF-8"))?
                    .to_owned();
                validate_text(&text)?;
                Ok(Value::Text(text))
            }
            4 => {
                let length = self.length(additional)?;
                let mut items = Vec::with_capacity(length);
                for _ in 0..length {
                    items.push(self.value()?);
                }
                Ok(Value::Array(items))
            }
            5 => {
                let length = self.length(additional)?;
                let mut entries = Vec::with_capacity(length);
                let mut keys = HashSet::new();
                for _ in 0..length {
                    let Value::Text(key) = self.value()? else {
                        return Err(Diagnostic::plain(
                            "BHCP3005",
                            "implemented artifact maps require text keys",
                        ));
                    };
                    if !keys.insert(key.clone()) {
                        return Err(Diagnostic::plain("BHCP3004", "duplicate CBOR map key"));
                    }
                    entries.push((key, self.value()?));
                }
                Ok(Value::owned_map(entries))
            }
            6 => {
                let tag = self.argument(additional)?;
                Ok(Value::Tag(tag, Box::new(self.value()?)))
            }
            7 if additional == 20 => Ok(Value::Bool(false)),
            7 if additional == 21 => Ok(Value::Bool(true)),
            7 if additional == 22 => Ok(Value::Null),
            _ => Err(Diagnostic::plain(
                "BHCP3005",
                "floats and unsupported simple CBOR values are forbidden",
            )),
        }
    }

    fn argument(&mut self, additional: u8) -> Result<u64> {
        match additional {
            0..=23 => Ok(u64::from(additional)),
            24 => Ok(u64::from(self.byte()?)),
            25 => Ok(u64::from(u16::from_be_bytes(
                self.take(2)?.try_into().unwrap(),
            ))),
            26 => Ok(u64::from(u32::from_be_bytes(
                self.take(4)?.try_into().unwrap(),
            ))),
            27 => Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap())),
            _ => Err(Diagnostic::plain(
                "BHCP3005",
                "invalid CBOR additional information",
            )),
        }
    }

    fn length(&mut self, additional: u8) -> Result<usize> {
        usize::try_from(self.argument(additional)?)
            .map_err(|_| Diagnostic::plain("BHCP3005", "CBOR length exceeds platform range"))
    }

    fn byte(&mut self) -> Result<u8> {
        let byte = *self
            .bytes
            .get(self.cursor)
            .ok_or_else(|| Diagnostic::plain("BHCP3005", "truncated CBOR"))?;
        self.cursor += 1;
        Ok(byte)
    }
    fn take(&mut self, length: usize) -> Result<&[u8]> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or_else(|| Diagnostic::plain("BHCP3005", "CBOR length overflow"))?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| Diagnostic::plain("BHCP3005", "truncated CBOR"))?;
        self.cursor = end;
        Ok(value)
    }
}
