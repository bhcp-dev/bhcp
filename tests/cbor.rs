use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::value::Value;

#[test]
fn deterministic_map_order_is_length_then_bytes() {
    let value = Value::map([("aa", Value::Integer(2)), ("b", Value::Integer(1))]);
    let bytes = encode_deterministic(&value).unwrap();
    assert_eq!(hex(&bytes), "a261620162616102");
    assert_eq!(decode_deterministic(&bytes).unwrap(), value);
}

#[test]
fn rejects_host_floats_indefinite_lengths_and_non_shortest_integers() {
    assert!(decode_deterministic(&[0xfb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0]).is_err());
    assert!(decode_deterministic(&[0x9f, 0xff]).is_err());
    assert!(decode_deterministic(&[0x18, 0x01]).is_err());
}

#[test]
fn deterministic_cbor_covers_the_full_integer_major_type_domain() {
    for value in [
        Value::Integer(i128::from(i64::MAX) + 1),
        Value::Integer(i128::from(u64::MAX)),
        Value::Integer(-1 - i128::from(u64::MAX)),
    ] {
        let bytes = encode_deterministic(&value).unwrap();
        assert_eq!(decode_deterministic(&bytes).unwrap(), value);
    }
    assert!(encode_deterministic(&Value::Integer(i128::from(u64::MAX) + 1)).is_err());
    assert!(encode_deterministic(&Value::Integer(-2 - i128::from(u64::MAX))).is_err());
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
