//! Codelists published under `urn:xoev-de:xwasser:codeliste:` (XWasser).
//!
//! Datasets embed every published version: the newest as a full snapshot, older
//! ones as reverse deltas ([`xoev_codelists::Dataset`]).

use std::sync::LazyLock;

use xoev_codelists::Catalog;

/// Namespace all URNs of this crate's codelists start with.
pub const NAMESPACE: &str = "urn:xoev-de:xwasser:codeliste:";

include!(concat!(env!("OUT_DIR"), "/datasets.rs"));

/// All datasets of this publisher.
pub fn catalog() -> &'static Catalog {
    static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
        Catalog::from_json(DATASETS.iter().copied()).expect("embedded datasets are valid")
    });
    &CATALOG
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_embedded_dataset_reconstructs_all_its_versions() {
        for document in super::DATASETS.iter().copied() {
            let dataset: xoev_codelists::Dataset =
                serde_json::from_str(document).expect("dataset json");
            let versions: Vec<String> = dataset.versions().map(str::to_string).collect();
            for version in &versions {
                let snapshot = dataset
                    .resolve(version)
                    .unwrap_or_else(|| panic!("{} {} not resolvable", dataset.urn, version));
                assert_eq!(&*snapshot.version, version.as_str());
            }
            assert!(dataset.base.urn.starts_with(super::NAMESPACE));
            assert!(versions.contains(&dataset.current_version().to_string()));
        }
    }
}

#[cfg(test)]
mod version_aware_tests {
    use super::catalog;

    const UNTERSUCHUNGSVERFAHREN: &str = "urn:xoev-de:xwasser:codeliste:untersuchungsverfahren";

    #[test]
    fn a_sender_may_use_an_older_version() {
        let catalog = catalog();

        // codes that existed in version 1 and are gone from version 3
        assert_eq!(
            catalog.contains(UNTERSUCHUNGSVERFAHREN, "1", "1760"),
            Some(true)
        );
        assert_eq!(
            catalog.contains(UNTERSUCHUNGSVERFAHREN, "3", "1760"),
            Some(false)
        );

        // codes that only exist since version 3
        assert_eq!(
            catalog.contains(UNTERSUCHUNGSVERFAHREN, "3", "3210"),
            Some(true)
        );
        assert_eq!(
            catalog.contains(UNTERSUCHUNGSVERFAHREN, "1", "3210"),
            Some(false)
        );

        // an unknown version is not "the code is invalid"
        assert_eq!(catalog.contains(UNTERSUCHUNGSVERFAHREN, "9", "1760"), None);
    }

    #[test]
    fn label_of_an_old_code_is_resolved_from_its_version() {
        let catalog = catalog();
        let snapshot = catalog
            .resolve(UNTERSUCHUNGSVERFAHREN, "1")
            .expect("version 1");
        assert!(
            snapshot
                .text("1760", "Untersuchungsverfahren")
                .is_some_and(|text| !text.is_empty()),
            "old code must still have a label"
        );
    }
}
