//! Codelists published under `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:` (Destatis).
//!
//! Datasets embed every published version: the newest as a full snapshot, older
//! ones as reverse deltas ([`xoev_codelists::Dataset`]).

use std::sync::LazyLock;

use xoev_codelists::Catalog;

/// Namespace all URNs of this crate's codelists start with.
pub const NAMESPACE: &str = "urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:";

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
