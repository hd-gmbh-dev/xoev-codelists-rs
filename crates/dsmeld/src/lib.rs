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

/// Label bundle: key column plus the label column per version.
#[cfg(all(feature = "labels", not(feature = "full")))]
pub static DATASETS: &[u8] = include_bytes!("../data/labels.zst");

/// Code-only bundle: per codelist and version just the set of valid codes.
#[cfg(all(feature = "codes", not(any(feature = "full", feature = "labels"))))]
pub static DATASETS: &[u8] = include_bytes!("../data/codes.zst");

#[cfg(not(any(feature = "full", feature = "labels", feature = "codes")))]
compile_error!("enable one of the `full`, `labels` or `codes` features");

/// Namespace all URNs of this crate's codelists start with.
pub const NAMESPACE: &str = "urn:de:dsmeld:schluesseltabelle:";

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
