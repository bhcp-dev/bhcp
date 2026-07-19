#[cfg(not(debug_assertions))]
compile_error!("controlled release-build gate failure for issue #12 evidence");

pub mod cbor;
pub mod diagnostic;
pub mod hash;
pub mod inspection;
pub mod kernel;
pub mod manifest;
pub mod model;
pub mod parser;
pub mod pipeline;
pub mod prelude;
pub mod schema;
pub mod value;
pub mod verification;
