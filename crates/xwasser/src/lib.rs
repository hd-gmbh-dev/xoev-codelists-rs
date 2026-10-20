//! Codelists published under `NAMESPACE`.
//!
//! Every published version is embedded: the newest as a full snapshot, older
//! ones as reverse deltas ([`xoev_codelists::Dataset`]). The datasets are
//! stored compressed as one bundle and decompressed on first use, so a binary
//! (and in particular a wasm build) only carries the compressed form.

use std::sync::LazyLock;

use xoev_codelists::Catalog;

/// Compressed dataset bundle, see [`xoev_codelists::decode_bundle`].
///
/// With the `codes` feature (and without `full`) this is the code-only bundle:
/// it validates transmitted codes but carries no labels.
#[cfg(feature = "full")]
pub static DATASETS: &[u8] = include_bytes!("../data/datasets.zst");

/// Code-only bundle: per codelist and version just the set of valid codes.
#[cfg(all(feature = "codes", not(feature = "full")))]
pub static DATASETS: &[u8] = include_bytes!("../data/codes.zst");

#[cfg(not(any(feature = "full", feature = "codes")))]
compile_error!("enable either the `full` or the `codes` feature");

/// Namespace all URNs of this crate's codelists start with.
pub const NAMESPACE: &str = "urn:xoev-de:xwasser:codeliste:";

/// All datasets of this publisher.
pub fn catalog() -> &'static Catalog {
    static CATALOG: LazyLock<Catalog> =
        LazyLock::new(|| Catalog::from_bundle(DATASETS).expect("embedded datasets are valid"));
    &CATALOG
}

#[cfg(test)]
mod tests {
    use super::{DATASETS, NAMESPACE, catalog};
    use xoev_codelists::{decode_bundle, split_bundle};

    #[test]
    fn every_embedded_dataset_reconstructs_all_its_versions() {
        let decoded = decode_bundle(DATASETS).expect("bundle");
        let documents = split_bundle(&decoded).expect("documents");
        for document in documents {
            let dataset: xoev_codelists::Dataset =
                serde_json::from_str(document).expect("dataset json");
            // fold the chain once and check that every declared version is
            // produced in order
            let versions: Vec<String> = dataset.versions().map(str::to_string).collect();
            let rebuilt: Vec<String> = dataset
                .reconstructions()
                .map(|snapshot| snapshot.version.to_string())
                .collect();
            assert_eq!(rebuilt, versions, "{}", dataset.urn);
            assert!(dataset.base.urn.starts_with(NAMESPACE));
        }
        assert!(catalog().urns().count() > 0);
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
    fn the_code_only_bundle_validates_but_has_no_labels() {
        if cfg!(feature = "full") {
            return;
        }
        let snapshot = catalog()
            .resolve(UNTERSUCHUNGSVERFAHREN, "1")
            .expect("version 1");
        assert_eq!(snapshot.columns.len(), 1, "codes bundle carries keys only");
        assert!(snapshot.contains_code("1760"));
    }

    #[test]
    fn label_of_an_old_code_is_resolved_from_its_version() {
        if !cfg!(feature = "full") {
            return;
        }
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
