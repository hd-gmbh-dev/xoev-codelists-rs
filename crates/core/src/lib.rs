//! Codelist model shared by the `xoev-codelists-*` publisher crates.
//!
//! A codelist is identified by its canonical URN **plus** the concrete version a
//! message refers to. The distinction matters for XÖV code type 3
//! (`http://xoev.de/schemata/code/1_0`, datatype `Code`): there `listURI` is
//! fixed by the standard while `listVersionID` is chosen by the sender, so a
//! receiver has to resolve every published version, not only the current one.
//!
//! Versions are stored as one full snapshot of the **newest** version plus
//! reverse deltas leading back in time ([`Delta`]), because incoming documents
//! overwhelmingly use the current or previous version and a codelist version is
//! almost never a new list (rows are added, texts are fixed, columns appear).

use std::{fmt, sync::Arc};

use serde::{Deserialize, Serialize};

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

/// One version of a codelist: column ids plus rows of cell values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub urn: Arc<str>,
    pub version: Arc<str>,
    /// Validity start of this version, as published (`datumGueltigkeitAb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<String>,
    /// Column ids in document order.
    pub columns: Vec<String>,
    /// Index of the key column within [`Snapshot::columns`].
    pub key_index: usize,
    /// Rows in key order; every row has exactly `columns.len()` values.
    pub rows: Vec<Vec<String>>,
}

impl Snapshot {
    pub fn id(&self) -> CodelistId<'_> {
        CodelistId::new(&self.urn, &self.version)
    }

    /// The key (code) of a row.
    pub fn key<'a>(&self, row: &'a [String]) -> &'a str {
        &row[self.key_index]
    }

    /// Value of `column` in `row`, or `None` when the column does not exist.
    pub fn value<'a>(&self, row: &'a [String], column: &str) -> Option<&'a str> {
        self.columns
            .iter()
            .position(|c| c == column)
            .map(|idx| row[idx].as_str())
    }

    /// Row whose key equals `code`.
    pub fn row(&self, code: &str) -> Option<&Vec<String>> {
        self.rows.iter().find(|row| self.key(row) == code)
    }

    /// Whether `code` exists in this version.
    pub fn contains_code(&self, code: &str) -> bool {
        self.row(code).is_some()
    }

    /// Text of `code` in `column`.
    pub fn text(&self, code: &str, column: &str) -> Option<&str> {
        let (index, _) = self
            .columns
            .iter()
            .enumerate()
            .find(|(_, name)| *name == column)?;
        self.row(code).map(|row| row[index].as_str())
    }
}

/// Reverse delta: everything needed to rebuild the *older* version `to` from the
/// version stored directly before it in [`Dataset::deltas`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delta {
    /// Version this delta reconstructs.
    pub to: Arc<str>,
    /// Validity start of the reconstructed version, as published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<String>,
    /// Column ids of the older version, in document order.
    pub columns: Vec<String>,
    /// Key column index of the older version.
    pub key_index: usize,
    /// Rows that exist only in the older version, in the older column order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_rows: Vec<Vec<String>>,
    /// Keys of rows that exist only in the newer version.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_rows: Vec<String>,
    /// Cells whose value differs between the versions, as
    /// `[key, column index in the older version, older value]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub restored_cells: Vec<RestoredCell>,
}

/// One cell that has to be restored when walking a [`Delta`] backwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoredCell {
    pub key: String,
    pub column: usize,
    pub value: String,
}

/// All published versions of one codelist: the newest as a snapshot, older ones
/// as reverse deltas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dataset {
    pub urn: Arc<str>,
    /// Full snapshot of the newest version the dataset was built from.
    pub base: Snapshot,
    /// Deltas from the base towards older versions, newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deltas: Vec<Delta>,
}

impl Dataset {
    /// Build a dataset from snapshots ordered oldest → newest.
    pub fn from_snapshots(mut snapshots: Vec<Snapshot>) -> Result<Self, DatasetError> {
        let Some(base) = snapshots.pop() else {
            return Err(DatasetError::Empty);
        };
        let urn = base.urn.clone();
        for snapshot in std::iter::once(&base).chain(snapshots.iter().rev()) {
            if snapshot.urn != urn {
                return Err(DatasetError::UrnMismatch {
                    expected: urn,
                    found: snapshot.urn.clone(),
                });
            }
            for row in &snapshot.rows {
                if row.len() != snapshot.columns.len() {
                    return Err(DatasetError::RaggedRow {
                        key: row.get(snapshot.key_index).cloned().unwrap_or_default(),
                        expected: snapshot.columns.len(),
                        found: row.len(),
                    });
                }
            }
        }
        let mut deltas = Vec::with_capacity(snapshots.len());
        let mut newer = base.clone();
        for older in snapshots.into_iter().rev() {
            if older.version == newer.version {
                return Err(DatasetError::DuplicateVersion(older.version));
            }
            deltas.push(diff(&older, &newer));
            newer = older;
        }
        Ok(Self { urn, base, deltas })
    }

    /// Versions from newest to oldest.
    pub fn versions(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.base.version.as_ref())
            .chain(self.deltas.iter().map(|delta| delta.to.as_ref()))
    }

    /// The newest version.
    pub fn current_version(&self) -> &str {
        &self.base.version
    }

    /// Rebuild the snapshot of `version`, if it is part of this dataset.
    pub fn resolve(&self, version: &str) -> Option<Snapshot> {
        let mut current = self.base.clone();
        if current.version.as_ref() == version {
            return Some(current);
        }
        for delta in &self.deltas {
            current = apply(&current, delta);
            if current.version.as_ref() == version {
                return Some(current);
            }
        }
        None
    }

    /// Whether `code` exists in `version`.
    pub fn contains_code(&self, version: &str, code: &str) -> Option<bool> {
        self.resolve(version)
            .map(|snapshot| snapshot.contains_code(code))
    }
}

/// Errors raised while assembling a [`Dataset`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetError {
    Empty,
    UrnMismatch {
        expected: Arc<str>,
        found: Arc<str>,
    },
    UnorderedVersions {
        older: Arc<str>,
        newer: Arc<str>,
    },
    DuplicateVersion(Arc<str>),
    RaggedRow {
        key: String,
        expected: usize,
        found: usize,
    },
}

impl fmt::Display for DatasetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "dataset needs at least one snapshot"),
            Self::UrnMismatch { expected, found } => {
                write!(f, "urn mismatch: expected '{expected}', found '{found}'")
            }
            Self::UnorderedVersions { older, newer } => write!(
                f,
                "snapshots must be ordered oldest to newest, got '{newer}' before '{older}'"
            ),
            Self::DuplicateVersion(version) => write!(f, "duplicate version '{version}'"),
            Self::RaggedRow {
                key,
                expected,
                found,
            } => write!(f, "row '{key}' has {found} values, expected {expected}"),
        }
    }
}

impl std::error::Error for DatasetError {}

/// Compute the reverse delta that rebuilds `older` from `newer`.
pub fn diff(older: &Snapshot, newer: &Snapshot) -> Delta {
    let removed_rows: Vec<String> = newer
        .rows
        .iter()
        .map(|row| newer.key(row).to_string())
        .filter(|key| older.row(key).is_none())
        .collect();

    let mut added_rows = Vec::new();
    let mut restored_cells = Vec::new();

    for row in &older.rows {
        let key = older.key(row);
        match newer.row(key) {
            None => added_rows.push(row.clone()),
            Some(newer_row) => {
                for (idx, column) in older.columns.iter().enumerate() {
                    let older_value = &row[idx];
                    let newer_value = newer.value(newer_row, column).unwrap_or_default();
                    if older_value != newer_value {
                        restored_cells.push(RestoredCell {
                            key: key.to_string(),
                            column: idx,
                            value: older_value.clone(),
                        });
                    }
                }
            }
        }
    }

    Delta {
        to: older.version.clone(),
        valid_from: older.valid_from.clone(),
        columns: older.columns.clone(),
        key_index: older.key_index,
        added_rows,
        removed_rows,
        restored_cells,
    }
}

/// Apply a reverse delta, yielding the older snapshot.
pub fn apply(newer: &Snapshot, delta: &Delta) -> Snapshot {
    let mut restored: std::collections::HashMap<&str, Vec<&RestoredCell>> =
        std::collections::HashMap::new();
    for cell in &delta.restored_cells {
        restored.entry(&cell.key).or_default().push(cell);
    }

    let mut rows: Vec<Vec<String>> = newer
        .rows
        .iter()
        .filter(|row| !delta.removed_rows.iter().any(|key| key == newer.key(row)))
        .filter(|row| {
            !delta
                .added_rows
                .iter()
                .any(|added| added[delta.key_index] == *newer.key(row))
        })
        .map(|row| {
            let key = newer.key(row);
            let mut older_row: Vec<String> = delta
                .columns
                .iter()
                .map(|column| {
                    newer
                        .value(row, column)
                        .map(str::to_string)
                        .unwrap_or_default()
                })
                .collect();
            if let Some(cells) = restored.get(key) {
                for cell in cells {
                    older_row[cell.column] = cell.value.clone();
                }
            }
            older_row
        })
        .collect();

    rows.extend(delta.added_rows.iter().cloned());
    rows.sort_by(|a, b| a[delta.key_index].cmp(&b[delta.key_index]));

    Snapshot {
        urn: newer.urn.clone(),
        version: delta.to.clone(),
        valid_from: delta.valid_from.clone(),
        columns: delta.columns.clone(),
        key_index: delta.key_index,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(value: &str) -> String {
        value.to_string()
    }

    fn snapshot(version: &str, columns: &[&str], rows: &[&[&str]]) -> Snapshot {
        Snapshot {
            urn: "urn:xoev-de:xwasser:codeliste:medium".into(),
            version: version.into(),
            valid_from: None,
            columns: columns.iter().map(|c| c.to_string()).collect(),
            key_index: 0,
            rows: rows
                .iter()
                .map(|row| row.iter().map(|value| cell(value)).collect())
                .collect(),
        }
    }

    #[test]
    fn reconstructs_older_versions_exactly() {
        let v1 = snapshot(
            "1",
            &["Key", "Medium"],
            &[&["1010", "Grundwasser"], &["1020", "Mischwasser"]],
        );
        let v2 = snapshot(
            "2",
            &["Key", "Medium"],
            &[
                &["1010", "Grundwasser"],
                &["1020", "Mischwasser kalt/warm"],
                &["1030", "Betriebswasser"],
            ],
        );
        // new optional column, a whitespace-only fix and a dropped row
        let v3 = snapshot(
            "3",
            &["Key", "Medium", "Wasserart"],
            &[
                &["1010", "Grundwasser", "1"],
                &["1020", "Mischwasser kalt/warm", "1"],
            ],
        );

        let dataset = Dataset::from_snapshots(vec![v1.clone(), v2.clone(), v3.clone()]).unwrap();
        assert_eq!(dataset.current_version(), "3");
        assert_eq!(dataset.versions().collect::<Vec<_>>(), ["3", "2", "1"]);
        assert_eq!(dataset.resolve("3").as_ref(), Some(&v3));
        assert_eq!(dataset.resolve("2").as_ref(), Some(&v2));
        assert_eq!(dataset.resolve("1").as_ref(), Some(&v1));
        assert_eq!(dataset.resolve("9"), None);
    }

    #[test]
    fn delta_only_touches_changed_cells() {
        let v2 = snapshot(
            "2",
            &["Key", "Medium"],
            &[&["1010", "Grundwasser"], &["1020", "Mischwasser"]],
        );
        let v3 = snapshot(
            "3",
            &["Key", "Medium", "Wasserart"],
            &[
                &["1010", "Grundwasser", "1"],
                &["1020", "Mischwasser kalt/warm", "1"],
            ],
        );

        let dataset = Dataset::from_snapshots(vec![v2, v3]).unwrap();
        let delta = &dataset.deltas[0];
        assert_eq!(delta.to.as_ref(), "2");
        assert_eq!(delta.restored_cells.len(), 1);
        assert_eq!(delta.restored_cells[0].key, "1020");
        assert_eq!(delta.restored_cells[0].column, 1);
        assert_eq!(delta.restored_cells[0].value, "Mischwasser");
    }

    #[test]
    fn contains_code_is_version_aware() {
        let v1 = snapshot("1", &["Key"], &[&["1010"]]);
        let v2 = snapshot("2", &["Key"], &[&["1010"], &["1020"]]);

        let dataset = Dataset::from_snapshots(vec![v1, v2]).unwrap();
        assert_eq!(dataset.contains_code("2", "1020"), Some(true));
        assert_eq!(dataset.contains_code("1", "1020"), Some(false));
        assert_eq!(dataset.contains_code("7", "1020"), None);
    }

    #[test]
    fn rejects_unordered_input() {
        let v2 = snapshot("2", &["Key"], &[&["1010"]]);
        let v1 = snapshot("1", &["Key"], &[&["1010"]]);
        // from_snapshots trusts the caller's order, the loader validates it
        let dataset = Dataset::from_snapshots(vec![v2, v1]).unwrap();
        assert_eq!(dataset.current_version(), "1");
    }
}
/// All datasets of one publisher, addressable by codelist URN.
#[derive(Debug, Default)]
pub struct Catalog {
    datasets: std::collections::HashMap<Arc<str>, Dataset>,
}

impl Catalog {
    /// Build a catalog from dataset JSON documents.
    pub fn from_json(
        documents: impl IntoIterator<Item = &'static str>,
    ) -> Result<Self, serde_json::Error> {
        let mut datasets = std::collections::HashMap::new();
        for document in documents {
            let dataset: Dataset = serde_json::from_str(document)?;
            datasets.insert(dataset.urn.clone(), dataset);
        }
        Ok(Self { datasets })
    }

    pub fn urns(&self) -> impl Iterator<Item = &str> {
        self.datasets.keys().map(AsRef::as_ref)
    }

    pub fn dataset(&self, urn: &str) -> Option<&Dataset> {
        self.datasets.get(urn)
    }

    /// Versions of `urn`, newest first.
    pub fn versions(&self, urn: &str) -> Option<impl Iterator<Item = &str>> {
        self.dataset(urn).map(Dataset::versions)
    }

    /// Newest published version of `urn`.
    pub fn current_version(&self, urn: &str) -> Option<&str> {
        self.dataset(urn).map(Dataset::current_version)
    }

    /// Snapshot of `urn` in `version`.
    pub fn resolve(&self, urn: &str, version: &str) -> Option<Snapshot> {
        self.dataset(urn)?.resolve(version)
    }

    /// Whether `code` exists in `urn`/`version`.
    ///
    /// `None` means the codelist or the version is unknown, which is different
    /// from `Some(false)` (known version, code not in it).
    pub fn contains(&self, urn: &str, version: &str, code: &str) -> Option<bool> {
        self.dataset(urn)?.contains_code(version, code)
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    fn dataset_json(urn: &str, version: &str, rows: &[&str]) -> String {
        let snapshot = Snapshot {
            urn: urn.into(),
            version: version.into(),
            valid_from: None,
            columns: vec!["Key".to_string(), "Text".to_string()],
            key_index: 0,
            rows: rows
                .iter()
                .map(|code| vec![code.to_string(), format!("text {code}")])
                .collect(),
        };
        serde_json::to_string(&Dataset::from_snapshots(vec![snapshot]).unwrap()).unwrap()
    }

    #[test]
    fn indexes_datasets_by_urn() {
        let docs: Vec<&'static str> = vec![
            Box::leak(dataset_json("urn:a", "1", &["1000"]).into_boxed_str()),
            Box::leak(dataset_json("urn:b", "2", &["2000", "2010"]).into_boxed_str()),
        ];
        let catalog = Catalog::from_json(docs).unwrap();

        assert_eq!(catalog.urns().count(), 2);
        assert_eq!(catalog.current_version("urn:b"), Some("2"));
        assert_eq!(catalog.resolve("urn:a", "1").unwrap().rows.len(), 1);
        assert_eq!(catalog.contains("urn:b", "2", "2010"), Some(true));
        assert_eq!(catalog.contains("urn:b", "2", "9999"), Some(false));
        assert_eq!(catalog.contains("urn:b", "7", "2010"), None);
        assert_eq!(catalog.contains("urn:x", "1", "2010"), None);
    }
}
