//! Codelists published by Destatis under `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:`.
//!
//! The datasets (base version plus reverse deltas per older version) are not
//! part of this alpha release; it reserves the crate name and the namespace
//! contract only.

/// Namespace all URNs of this crate's codelists start with.
pub const NAMESPACE: &str = "urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:";

#[cfg(test)]
mod tests {
    #[test]
    fn namespace_is_a_prefix_of_a_known_urn() {
        assert!(
            "urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:ags"
                .starts_with(super::NAMESPACE)
        );
    }
}
