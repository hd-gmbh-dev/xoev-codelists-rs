//! Proves that the bundles behave exactly as documented:
//!
//! * the code-only bundle validates exactly the codes the full bundle knows,
//!   for every codelist and every published version;
//! * a code that was dropped between versions is recognised in its own version
//!   and rejected in the newer one;
//! * unknown codelists and unknown versions are reported as unknown, not as
//!   "invalid code";
//! * published versions are ordered by validity, duplicates and unsorted input
//!   survive reconstruction.

use std::collections::BTreeSet;

use xoev_codelists::{Catalog, Dataset};

macro_rules! bundles {
    ($($publisher:literal: $full:literal, $codes:literal),* $(,)?) => {
        fn publishers() -> Vec<(&'static str, &'static [u8], &'static [u8])> {
            vec![$(($publisher, include_bytes!($full), include_bytes!($codes))),*]
        }
    };
}

bundles![
    "xwasser": "../../xwasser/data/datasets.zst", "../../xwasser/data/codes.zst",
    "destatis": "../../destatis/data/datasets.zst", "../../destatis/data/codes.zst",
    "xunternehmen": "../../xunternehmen/data/datasets.zst", "../../xunternehmen/data/codes.zst",
    "dsmeld": "../../dsmeld/data/datasets.zst", "../../dsmeld/data/codes.zst",
    "xauslaender": "../../xauslaender/data/datasets.zst", "../../xauslaender/data/codes.zst",
    "bmi-bit": "../../bmi-bit/data/datasets.zst", "../../bmi-bit/data/codes.zst",
];

fn catalogs() -> Vec<(&'static str, Catalog, Catalog)> {
    publishers()
        .into_iter()
        .map(|(name, full, codes)| {
            (
                name,
                Catalog::from_bundle(full).unwrap_or_else(|err| panic!("{name} full: {err}")),
                Catalog::from_bundle(codes).unwrap_or_else(|err| panic!("{name} codes: {err}")),
            )
        })
        .collect()
}

fn code_set(snapshot: &xoev_codelists::Snapshot) -> BTreeSet<String> {
    snapshot
        .rows
        .iter()
        .map(|row| snapshot.key(row).to_string())
        .collect()
}

/// The code-only bundle must carry exactly the codes of the full bundle, in the
/// same version order, for every codelist of every publisher.
#[test]
fn code_only_bundle_matches_full_bundle() {
    let mut codelists = 0;
    let mut versions = 0;
    let mut codes = 0;

    for (publisher, full, codes_catalog) in catalogs() {
        let mut full_urns: Vec<&str> = full.urns().collect();
        let mut codes_urns: Vec<&str> = codes_catalog.urns().collect();
        full_urns.sort();
        codes_urns.sort();
        assert_eq!(full_urns, codes_urns, "{publisher}: different codelists");

        for urn in full_urns {
            let full_versions: Vec<&str> = full.versions(urn).unwrap().collect();
            let codes_versions: Vec<&str> = codes_catalog.versions(urn).unwrap().collect();
            assert_eq!(full_versions, codes_versions, "{urn}: different versions");

            let full_snapshots: Vec<_> = full.dataset(urn).unwrap().reconstructions().collect();
            let codes_snapshots: Vec<_> = codes_catalog
                .dataset(urn)
                .unwrap()
                .reconstructions()
                .collect();
            assert_eq!(full_snapshots.len(), codes_snapshots.len(), "{urn}");

            for (full_snapshot, codes_snapshot) in full_snapshots.iter().zip(codes_snapshots.iter())
            {
                assert_eq!(full_snapshot.version, codes_snapshot.version, "{urn}");
                let full_codes = code_set(full_snapshot);
                let codes_only = code_set(codes_snapshot);
                assert_eq!(
                    full_codes, codes_only,
                    "{urn} {}: code set differs between bundles",
                    full_snapshot.version
                );
                codes += full_codes.len();
                versions += 1;
            }
            codelists += 1;
        }
    }

    // keep the test honest about its coverage
    assert_eq!(codelists, 27, "expected all 27 type-3 codelists");
    assert!(
        versions >= 272,
        "expected at least 272 versions, got {versions}"
    );
    assert!(codes > 100_000, "expected a large code corpus, got {codes}");
    println!("checked {codelists} codelists, {versions} versions, {codes} codes");
}

/// For every codelist, codes that were dropped between the oldest and the newest
/// version must be valid in the old version and invalid in the new one.
#[test]
fn dropped_codes_are_version_aware_everywhere() {
    let mut codelists_with_drops = 0;
    let mut checked = 0;

    for (publisher, full, _) in catalogs() {
        for urn in full.urns() {
            let dataset = full.dataset(urn).unwrap();
            let versions: Vec<&str> = dataset.versions().collect();
            let (Some(oldest), Some(newest)) = (versions.last(), versions.first()) else {
                continue;
            };
            let old_snapshot = full.resolve(urn, oldest).unwrap();
            let new_snapshot = full.resolve(urn, newest).unwrap();
            let old_codes = code_set(&old_snapshot);
            let new_codes = code_set(&new_snapshot);
            let dropped: Vec<&String> = old_codes.difference(&new_codes).collect();

            if dropped.is_empty() {
                continue;
            }
            codelists_with_drops += 1;
            for code in dropped.iter().take(20) {
                assert_eq!(
                    full.contains(urn, oldest, code),
                    Some(true),
                    "{publisher} {urn}: {code} was valid in {oldest}"
                );
                assert_eq!(
                    full.contains(urn, newest, code),
                    Some(false),
                    "{publisher} {urn}: {code} must be gone in {newest}"
                );
                checked += 1;
            }
            // and the other direction: new codes are not valid in the old version
            for code in new_codes.difference(&old_codes).take(20) {
                assert_eq!(
                    full.contains(urn, oldest, code),
                    Some(false),
                    "{urn} {code}"
                );
                assert_eq!(full.contains(urn, newest, code), Some(true), "{urn} {code}");
                checked += 1;
            }
        }
    }

    assert!(
        codelists_with_drops >= 10,
        "expected several codelists to have dropped codes, got {codelists_with_drops}"
    );
    println!("checked {checked} version-specific codes in {codelists_with_drops} codelists");
}

/// Unknown codelists and unknown versions are unknown - never "invalid code".
#[test]
fn unknown_urn_and_version_are_none() {
    for (_, full, codes) in catalogs() {
        for catalog in [&full, &codes] {
            assert_eq!(
                catalog.contains("urn:xoev-de:xwasser:codeliste:nope", "1", "1010"),
                None
            );
            assert_eq!(
                catalog.contains(
                    "urn:xoev-de:xwasser:codeliste:untersuchungsverfahren",
                    "99",
                    "1010"
                ),
                None
            );
            assert!(
                catalog
                    .resolve("urn:xoev-de:xwasser:codeliste:nope", "1")
                    .is_none()
            );
            assert!(
                catalog
                    .versions("urn:xoev-de:xwasser:codeliste:nope")
                    .is_none()
            );
        }
    }
}

/// Versions are ordered by validity date, independent of what the API returns.
#[test]
fn versions_are_ordered_and_unique() {
    for (publisher, full, _) in catalogs() {
        for urn in full.urns() {
            let dataset = full.dataset(urn).unwrap();
            let versions: Vec<String> = dataset.versions().map(str::to_string).collect();
            let unique: BTreeSet<&String> = versions.iter().collect();
            assert_eq!(
                unique.len(),
                versions.len(),
                "{publisher} {urn}: duplicate versions"
            );

            let dates: Vec<String> = dataset
                .reconstructions()
                .map(|snapshot| snapshot.valid_from.clone().unwrap_or_default())
                .collect();
            let mut sorted = dates.clone();
            sorted.sort_unstable();
            sorted.reverse();
            assert_eq!(
                dates, sorted,
                "{publisher} {urn}: versions not ordered by validity"
            );
        }
    }
}

/// Rows are canonicalised on load: a published version that is not sorted by key
/// is sorted, duplicate keys survive, and every step reconstructs exactly.
#[test]
fn unsorted_and_duplicate_keys_survive() {
    let destatis = Catalog::from_bundle(
        publishers()
            .into_iter()
            .find(|(name, _, _)| *name == "destatis")
            .unwrap()
            .1,
    )
    .unwrap();

    let staat = "urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:staat";
    let snapshot = destatis.resolve(staat, "2011-09-01").expect("2011-09-01");
    println!("2011-09-01: {} rows", snapshot.rows.len());

    // key 168 exists twice in 2011 and both rows are preserved
    let duplicates = snapshot
        .rows
        .iter()
        .filter(|row| snapshot.key(row) == "168")
        .count();
    assert_eq!(duplicates, 2, "duplicate key 168 must be preserved");
    assert_eq!(destatis.contains(staat, "2011-09-01", "168"), Some(true));

    // unsorted AGS versions are canonicalised: every version is sorted by key
    let ags = "urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:ags";
    let mut checked = 0;
    for snapshot in destatis.dataset(ags).unwrap().reconstructions() {
        let keys: Vec<&str> = snapshot.rows.iter().map(|row| snapshot.key(row)).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(
            keys, sorted,
            "AGS {} is not in canonical order",
            snapshot.version
        );
        checked += 1;
    }
    assert!(
        checked > 100,
        "expected the full AGS history, got {checked} versions"
    );
}

/// Every bundle must be decodable and every dataset must round-trip through
/// serialisation, so a consumer can persist and reload a dataset.
#[test]
fn datasets_round_trip_through_json() {
    for (publisher, full, _) in catalogs() {
        for urn in full.urns() {
            let dataset = full.dataset(urn).unwrap();
            let json = serde_json::to_string(dataset).unwrap();
            let reloaded: Dataset = serde_json::from_str(&json).unwrap();
            assert_eq!(&reloaded, dataset, "{publisher} {urn}");
        }
    }
}
