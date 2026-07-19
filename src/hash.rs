use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::model::{HashId, SemanticIrDocument};
use crate::value::Value;
use sha3::{Digest, Sha3_512};
use std::io::Read;

pub const SHA3_512: &str = "bhcp.hash/sha3-512@0";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HashAlgorithm {
    #[default]
    Sha3_512,
}

impl HashAlgorithm {
    pub fn from_id(id: &str) -> Result<Self> {
        match id {
            SHA3_512 => Ok(Self::Sha3_512),
            _ => Err(Diagnostic::plain(
                "BHCP6001",
                format!("project selects unregistered identity algorithm {id:?}"),
            )),
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Sha3_512 => SHA3_512,
        }
    }
    pub fn hash(self, bytes: &[u8]) -> HashId {
        let digest = match self {
            Self::Sha3_512 => Sha3_512::digest(bytes).to_vec(),
        };
        HashId {
            algorithm: self.id().to_owned(),
            digest,
        }
    }
}

pub fn hash_reader(mut reader: impl Read) -> std::io::Result<HashId> {
    let mut hasher = Sha3_512::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(HashId {
        algorithm: SHA3_512.to_owned(),
        digest: hasher.finalize().to_vec(),
    })
}

pub fn hash_value(value: &Value, algorithm: HashAlgorithm) -> Result<HashId> {
    Ok(algorithm.hash(&encode_deterministic(value)?))
}

pub fn semantic_hash_with(
    document: &SemanticIrDocument,
    algorithm: HashAlgorithm,
) -> Result<HashId> {
    hash_value(&document.semantic_value(), algorithm)
}

pub fn semantic_hash(document: &SemanticIrDocument) -> Result<HashId> {
    let algorithm = document
        .semantic_id
        .as_ref()
        .map(|hash| HashAlgorithm::from_id(&hash.algorithm))
        .transpose()?
        .unwrap_or_default();
    semantic_hash_with(document, algorithm)
}

pub fn artifact_hash_with(value: &Value, algorithm: HashAlgorithm) -> Result<HashId> {
    let mut value = value.clone();
    if let Value::Map(entries) = &mut value {
        entries.retain(|(key, _)| key != "artifact_id");
    }
    hash_value(&value, algorithm)
}

pub fn artifact_hash(value: &Value) -> Result<HashId> {
    artifact_hash_with(value, HashAlgorithm::default())
}

pub fn format_hash(hash: &HashId) -> String {
    format!(
        "{}:{}",
        hash.algorithm,
        hash.digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::{HashAlgorithm, format_hash, hash_reader};

    #[test]
    fn streaming_hash_matches_the_in_memory_identity() {
        let bytes = b"a deterministic input split across reader buffers";
        assert_eq!(
            hash_reader(bytes.as_slice()).unwrap(),
            HashAlgorithm::Sha3_512.hash(bytes)
        );
    }

    #[test]
    fn sha3_512_matches_known_vectors() {
        assert_eq!(
            format_hash(&HashAlgorithm::Sha3_512.hash(b"")),
            "bhcp.hash/sha3-512@0:a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26"
        );
        assert_eq!(
            format_hash(&HashAlgorithm::Sha3_512.hash(b"abc")),
            "bhcp.hash/sha3-512@0:b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0"
        );
    }
}
