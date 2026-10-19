//! Codelists published by XUnternehmen under `urn:xoev-de:xunternehmen:codeliste:`.
//!
//! The datasets (base version plus reverse deltas per older version) are not
//! part of this alpha release; it reserves the crate name and the namespace
//! contract only.

/// Namespace all URNs of this crate's codelists start with.
pub const NAMESPACE: &str = "urn:xoev-de:xunternehmen:codeliste:";

#[cfg(test)]
mod tests {
    #[test]
    fn namespace_is_a_prefix_of_a_known_urn() {
        assert!("urn:xoev-de:xunternehmen:codeliste:rechtsformen".starts_with(super::NAMESPACE));
    }
}
