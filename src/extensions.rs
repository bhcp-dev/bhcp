//! Closed extension support registered by exact semantic name and payload schema.
//!
//! Derived extensions are ordinary checked BHCP definitions and therefore do not
//! need a host registration. Native extensions remain must-understand IR nodes;
//! accepting one requires an explicit implementation registration whose schema
//! reference exactly matches the source descriptor.

use std::collections::BTreeMap;

use crate::cbor::encode_deterministic;
use crate::diagnostic::{Diagnostic, Result};
use crate::model::is_symbol;
use crate::policy::ArtifactReference;
use crate::value::Value;

const INVALID_EXTENSION: &str = "BHCP5003";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeExtensionRegistration {
    pub payload_schema: Value,
    pub payload: Value,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExtensionRegistry {
    native: BTreeMap<String, NativeExtensionRegistration>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_native(
        &mut self,
        symbol: &str,
        payload_schema: Value,
        payload: Value,
    ) -> Result<()> {
        if !is_symbol(symbol) || symbol.starts_with("bhcp/") {
            return Err(invalid(
                "native extension registration requires a non-core symbol-id",
            ));
        }
        ArtifactReference::from_value(&payload_schema).map_err(|diagnostic| {
            invalid(format!(
                "native extension payload schema is invalid: {}",
                diagnostic.message
            ))
        })?;
        encode_deterministic(&payload).map_err(|diagnostic| {
            invalid(format!(
                "native extension payload is not deterministic CBOR: {}",
                diagnostic.message
            ))
        })?;
        if self.native.contains_key(symbol) {
            return Err(invalid(format!(
                "duplicate native extension registration {symbol:?}"
            )));
        }
        self.native.insert(
            symbol.to_owned(),
            NativeExtensionRegistration {
                payload_schema,
                payload,
            },
        );
        Ok(())
    }

    pub(crate) fn native(&self, symbol: &str) -> Option<&NativeExtensionRegistration> {
        self.native.get(symbol)
    }
}

pub(crate) fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_EXTENSION, message)
}
