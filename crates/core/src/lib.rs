//! Codelist model shared by the `xoev-codelists-*` publisher crates.
//!
//! A codelist is identified by its canonical URN **plus** the concrete version a
//! message refers to. The distinction matters for XÖV code type 3
//! (`http://xoev.de/schemata/code/1_0`, datatype `Code`): there `listURI` is
//! fixed by the standard while `listVersionID` is chosen by the sender, so a
//! receiver has to resolve every published version, not only the current one.

use std::fmt;

/// A codelist URN together with the version it is used in.
///
/// Renders as the canonical version URI, e.g.
/// `urn:xoev-de:xwasser:codeliste:untersuchungsverfahren_3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodelistId<'a> {
    pub urn: &'a str,
    pub version: &'a str,
}

impl<'a> CodelistId<'a> {
    pub const fn new(urn: &'a str, version: &'a str) -> Self {
        Self { urn, version }
    }
}

impl fmt::Display for CodelistId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.urn, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::CodelistId;

    #[test]
    fn renders_canonical_version_uri() {
        let id = CodelistId::new("urn:xoev-de:xwasser:codeliste:untersuchungsverfahren", "3");
        assert_eq!(
            id.to_string(),
            "urn:xoev-de:xwasser:codeliste:untersuchungsverfahren_3"
        );
    }
}
