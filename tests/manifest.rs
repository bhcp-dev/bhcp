use bhcp::hash::{HashAlgorithm, SHA3_512};
use bhcp::manifest::ProjectManifest;

#[test]
fn defaults_to_sha3_and_rejects_unregistered_algorithms() {
    assert_eq!(
        ProjectManifest::parse("", "manifest")
            .unwrap()
            .identity_algorithm,
        HashAlgorithm::Sha3_512
    );
    assert_eq!(
        ProjectManifest::parse(&format!("identity_algorithm = \"{SHA3_512}\""), "manifest")
            .unwrap()
            .identity_algorithm,
        HashAlgorithm::Sha3_512
    );
    let error =
        ProjectManifest::parse("identity_algorithm = \"example/hash@0\"", "manifest").unwrap_err();
    assert_eq!(error.code, "BHCP6001");
}
