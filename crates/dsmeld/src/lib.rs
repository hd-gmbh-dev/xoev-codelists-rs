//! Codelists published by DSMeld under `urn:de:dsmeld:schluesseltabelle:`.
//!
//! The datasets (base version plus reverse deltas per older version) are not
//! part of this alpha release; it reserves the crate name and the namespace
//! contract only.

/// Namespace all URNs of this crate's codelists start with.
pub const NAMESPACE: &str = "urn:de:dsmeld:schluesseltabelle:";

#[cfg(test)]
mod tests {
    #[test]
    fn namespace_is_a_prefix_of_a_known_urn() {
        assert!(
            "urn:de:dsmeld:schluesseltabelle:pass.und.ausweisdokumente"
                .starts_with(super::NAMESPACE)
        );
    }
}
