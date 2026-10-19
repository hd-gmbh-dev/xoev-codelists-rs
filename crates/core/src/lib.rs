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

use std::{fmt, io::Read as _, sync::Arc};

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

    /// Canonical order: rows sorted by key (stable, so duplicate keys keep
    /// their published order).
    ///
    /// Published codelists are not always sorted (some Destatis versions are
    /// not) and key-based deltas need one deterministic order.
    pub fn canonicalized(mut self) -> Result<Self, DatasetError> {
        self.rows
            .sort_by(|a, b| a[self.key_index].cmp(&b[self.key_index]));
        Ok(self)
    }

    /// Stable identity of every row in canonical order.
    ///
    /// A published codelist occasionally contains the same key twice (Destatis
    /// `staat` 2011 has two rows for key `168`), so rows are identified by key
    /// plus their occurrence among equal keys.
    pub fn row_ids(&self) -> Vec<String> {
        let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        self.rows
            .iter()
            .map(|row| {
                let key = self.key(row);
                let occurrence = seen.entry(key).or_insert(0);
                let id = if *occurrence == 0 {
                    key.to_string()
                } else {
                    format!("{key}\x01{occurrence:06}")
                };
                *occurrence += 1;
                id
            })
            .collect()
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
    /// Columns that were renamed between the versions (`from` = older name,
    /// `to` = name in the newer version). Without this a rename would show up
    /// as a changed cell in every single row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub renamed_columns: Vec<RenamedColumn>,
}

/// A column that changed its name between two versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamedColumn {
    pub from: String,
    pub to: String,
}

/// Minimum share of equal values for two columns to be treated as a rename.
const RENAME_THRESHOLD: f64 = 0.9;

/// One cell that has to be restored when walking a [`Delta`] backwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoredCell {
    pub key: String,
    pub column: usize,
    pub value: String,
}

/// One step backwards in time: usually a reverse delta, or a full snapshot when
/// the delta would not be smaller than the snapshot itself (a version that
/// renumbers its keys, for example).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Delta(Delta),
    Snapshot(Snapshot),
}

impl Step {
    pub fn version(&self) -> &str {
        match self {
            Self::Delta(delta) => &delta.to,
            Self::Snapshot(snapshot) => &snapshot.version,
        }
    }
}

/// All published versions of one codelist: the newest as a snapshot, older ones
/// as reverse deltas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dataset {
    pub urn: Arc<str>,
    /// Full snapshot of the newest version the dataset was built from.
    pub base: Snapshot,
    /// Steps from the base towards older versions, newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<Step>,
}

impl Dataset {
    /// Build a dataset from snapshots ordered oldest → newest.
    pub fn from_snapshots(snapshots: Vec<Snapshot>) -> Result<Self, DatasetError> {
        let mut snapshots: Vec<Snapshot> = snapshots
            .into_iter()
            .map(Snapshot::canonicalized)
            .collect::<Result<_, _>>()?;
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
        let mut steps = Vec::with_capacity(snapshots.len());
        let mut newer = base.clone();
        for older in snapshots.into_iter().rev() {
            if older.version == newer.version {
                return Err(DatasetError::DuplicateVersion(older.version));
            }
            // A delta that is not smaller than the snapshot it rebuilds is not
            // worth its complexity - store the snapshot instead.
            let delta = diff(&older, &newer);
            let step = if encoded_size(&delta) < encoded_size(&older) {
                Step::Delta(delta)
            } else {
                Step::Snapshot(older.clone())
            };
            steps.push(step);
            newer = older;
        }
        Ok(Self { urn, base, steps })
    }

    /// Versions from newest to oldest.
    pub fn versions(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.base.version.as_ref()).chain(self.steps.iter().map(Step::version))
    }

    /// The newest version.
    pub fn current_version(&self) -> &str {
        &self.base.version
    }

    /// Rebuild every version, newest first, folding the chain once.
    ///
    /// Cheaper than calling [`Dataset::resolve`] per version: O(steps) instead
    /// of O(steps^2).
    pub fn reconstructions(&self) -> impl Iterator<Item = Snapshot> + '_ {
        let mut next: Option<(usize, Snapshot)> = Some((0, self.base.clone()));
        std::iter::from_fn(move || {
            let (index, snapshot) = next.take()?;
            if let Some(step) = self.steps.get(index) {
                let older = match step {
                    Step::Delta(delta) => apply(&snapshot, delta),
                    Step::Snapshot(older) => older.clone(),
                };
                next = Some((index + 1, older));
            }
            Some(snapshot)
        })
    }

    /// Rebuild the snapshot of `version`, if it is part of this dataset.
    pub fn resolve(&self, version: &str) -> Option<Snapshot> {
        let mut current = self.base.clone();
        if current.version.as_ref() == version {
            return Some(current);
        }
        for step in &self.steps {
            current = match step {
                Step::Delta(delta) => apply(&current, delta),
                Step::Snapshot(snapshot) => snapshot.clone(),
            };
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

/// Size a value has in the dataset encoding, used to pick the cheaper step.
fn encoded_size(value: &impl Serialize) -> usize {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX)
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
    let renamed_columns = detect_renamed_columns(older, newer);
    let alias: std::collections::HashMap<&str, &str> = renamed_columns
        .iter()
        .map(|renamed| (renamed.from.as_str(), renamed.to.as_str()))
        .collect();

    let older_ids = older.row_ids();
    let newer_ids = newer.row_ids();
    let newer_by_id: std::collections::HashMap<&str, &[String]> = newer_ids
        .iter()
        .map(String::as_str)
        .zip(newer.rows.iter().map(Vec::as_slice))
        .collect();

    let removed_rows: Vec<String> = newer_ids
        .iter()
        .filter(|id| !older_ids.contains(id))
        .cloned()
        .collect();

    let mut added_rows = Vec::new();
    let mut restored_cells = Vec::new();

    for (row, id) in older.rows.iter().zip(&older_ids) {
        match newer_by_id.get(id.as_str()) {
            None => added_rows.push(row.clone()),
            Some(newer_row) => {
                for (idx, column) in older.columns.iter().enumerate() {
                    let older_value = &row[idx];
                    let source = alias.get(column.as_str()).copied().unwrap_or(column);
                    let newer_value = newer.value(newer_row, source).unwrap_or_default();
                    if older_value != newer_value {
                        restored_cells.push(RestoredCell {
                            key: id.clone(),
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
        renamed_columns,
    }
}

/// Pair columns that exist in only one of the versions when their values are
/// (almost) identical for the shared keys — i.e. the column was renamed.
fn detect_renamed_columns(older: &Snapshot, newer: &Snapshot) -> Vec<RenamedColumn> {
    let older_only: Vec<&str> = older
        .columns
        .iter()
        .map(String::as_str)
        .filter(|column| !newer.columns.iter().any(|name| name == column))
        .collect();
    let newer_only: Vec<&str> = newer
        .columns
        .iter()
        .map(String::as_str)
        .filter(|column| !older.columns.iter().any(|name| name == column))
        .collect();
    if older_only.is_empty() || newer_only.is_empty() {
        return Vec::new();
    }

    let newer_rows: std::collections::HashMap<&str, &[String]> = newer
        .rows
        .iter()
        .map(|row| (newer.key(row), row.as_slice()))
        .collect();

    let mut candidates: Vec<(f64, &str, &str)> = Vec::new();
    for older_column in &older_only {
        for newer_column in &newer_only {
            let mut shared = 0usize;
            let mut equal = 0usize;
            for row in &older.rows {
                let Some(newer_row) = newer_rows.get(older.key(row)) else {
                    continue;
                };
                shared += 1;
                let older_value = older.value(row, older_column).unwrap_or_default();
                let newer_value = newer.value(newer_row, newer_column).unwrap_or_default();
                if older_value == newer_value {
                    equal += 1;
                }
            }
            if shared == 0 {
                continue;
            }
            let ratio = equal as f64 / shared as f64;
            if ratio >= RENAME_THRESHOLD {
                candidates.push((ratio, older_column, newer_column));
            }
        }
    }

    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut taken_older: Vec<&str> = Vec::new();
    let mut taken_newer: Vec<&str> = Vec::new();
    let mut renamed = Vec::new();
    for (_, older_column, newer_column) in candidates {
        if taken_older.contains(&older_column) || taken_newer.contains(&newer_column) {
            continue;
        }
        taken_older.push(older_column);
        taken_newer.push(newer_column);
        renamed.push(RenamedColumn {
            from: older_column.to_string(),
            to: newer_column.to_string(),
        });
    }
    renamed
}

/// Apply a reverse delta, yielding the older snapshot.
pub fn apply(newer: &Snapshot, delta: &Delta) -> Snapshot {
    let alias: std::collections::HashMap<&str, &str> = delta
        .renamed_columns
        .iter()
        .map(|renamed| (renamed.from.as_str(), renamed.to.as_str()))
        .collect();

    let mut restored: std::collections::HashMap<&str, Vec<&RestoredCell>> =
        std::collections::HashMap::new();
    for cell in &delta.restored_cells {
        restored.entry(&cell.key).or_default().push(cell);
    }

    let mut entries: Vec<(String, Vec<String>)> = newer
        .rows
        .iter()
        .zip(newer.row_ids())
        .filter(|(_, id)| !delta.removed_rows.contains(id))
        .map(|(row, id)| {
            let mut older_row: Vec<String> = delta
                .columns
                .iter()
                .map(|column| {
                    let source = alias.get(column.as_str()).copied().unwrap_or(column);
                    newer
                        .value(row, source)
                        .map(str::to_string)
                        .unwrap_or_default()
                })
                .collect();
            if let Some(cells) = restored.get(id.as_str()) {
                for cell in cells {
                    older_row[cell.column] = cell.value.clone();
                }
            }
            (id, older_row)
        })
        .collect();

    // Rows only present in the older version carry their own identity; the sort
    // below restores canonical (key, occurrence) order.
    for row in &delta.added_rows {
        entries.push((row[delta.key_index].clone(), row.clone()));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    Snapshot {
        urn: newer.urn.clone(),
        version: delta.to.clone(),
        valid_from: delta.valid_from.clone(),
        columns: delta.columns.clone(),
        key_index: delta.key_index,
        rows: entries.into_iter().map(|(_, row)| row).collect(),
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
        let Step::Delta(delta) = &dataset.steps[0] else {
            panic!("expected a delta step");
        };
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
    fn renamed_column_does_not_change_every_cell() {
        let v2 = snapshot(
            "2",
            &["SHATPH-Parameter-ID", "Parameter", "Einheit"],
            &[&["10000", "Farbe", "nm"], &["10001", "Trübung", "NTU"]],
        );
        // typo fixed in the column name, one value corrected
        let v3 = snapshot(
            "3",
            &["SHAPTH-Parameter-ID", "Parameter", "Einheit"],
            &[&["10000", "Farbe", "nm"], &["10001", "Trübung", "FNU"]],
        );

        let delta = diff(&v2, &v3);
        assert_eq!(
            delta.renamed_columns,
            vec![RenamedColumn {
                from: "SHATPH-Parameter-ID".to_string(),
                to: "SHAPTH-Parameter-ID".to_string(),
            }]
        );
        assert_eq!(delta.restored_cells.len(), 1);
        assert_eq!(delta.restored_cells[0].key, "10001");

        let dataset = Dataset::from_snapshots(vec![v2.clone(), v3]).unwrap();
        assert_eq!(dataset.resolve("2").as_ref(), Some(&v2));
    }

    #[test]
    fn unrelated_columns_are_not_treated_as_renames() {
        let v2 = snapshot("2", &["Key", "Alt"], &[&["1010", "a"], &["1020", "b"]]);
        let v3 = snapshot("3", &["Key", "Neu"], &[&["1010", "x"], &["1020", "y"]]);

        let delta = diff(&v2, &v3);
        assert!(delta.renamed_columns.is_empty());
        assert_eq!(delta.restored_cells.len(), 2);

        let dataset = Dataset::from_snapshots(vec![v2.clone(), v3]).unwrap();
        assert_eq!(dataset.resolve("2").as_ref(), Some(&v2));
    }

    #[test]
    fn falls_back_to_a_snapshot_when_the_delta_is_not_smaller() {
        // every key is renumbered, so a delta has to restate all rows and ends
        // up bigger than the snapshot it rebuilds
        let build = |version: &str, key: &str| Snapshot {
            urn: "urn:xoev-de:xwasser:codeliste:medium".into(),
            version: version.into(),
            valid_from: None,
            columns: vec![key.to_string(), "Text".to_string()],
            key_index: 0,
            rows: (0..50)
                .map(|index| {
                    vec![
                        format!("{key}-{index}"),
                        format!("Bezeichnung des Mediums Nummer {index}"),
                    ]
                })
                .collect(),
        };
        let older = build("1", "Nummer");
        let newer = build("2", "Key");

        let expected = older.clone().canonicalized().unwrap();
        let dataset = Dataset::from_snapshots(vec![older, newer]).unwrap();
        assert!(
            matches!(dataset.steps[0], Step::Snapshot(_)),
            "renumbered keys must not produce a larger-than-snapshot delta"
        );
        assert_eq!(dataset.resolve("1").as_ref(), Some(&expected));
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
    pub fn from_json<'a>(
        documents: impl IntoIterator<Item = &'a str>,
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

/// Errors from loading a compressed dataset bundle.
#[derive(Debug)]
pub enum BundleError {
    Decode(std::io::Error),
    Format(&'static str),
    Json(serde_json::Error),
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(err) => write!(f, "cannot decompress dataset bundle: {err}"),
            Self::Format(message) => write!(f, "corrupt dataset bundle: {message}"),
            Self::Json(err) => write!(f, "invalid dataset json: {err}"),
        }
    }
}

impl std::error::Error for BundleError {}

/// Decode a bundle produced by the sync tool: zstd-compressed, a sequence of
/// `[u32 little endian length][json document]`.
pub fn decode_bundle(bundle: &[u8]) -> Result<Vec<u8>, BundleError> {
    let mut output = Vec::new();
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(bundle)
        .map_err(|err| BundleError::Decode(std::io::Error::other(err.to_string())))?;
    decoder
        .read_to_end(&mut output)
        .map_err(BundleError::Decode)?;
    Ok(output)
}

/// Split a decoded bundle into its JSON documents.
pub fn split_bundle(mut decoded: &[u8]) -> Result<Vec<&str>, BundleError> {
    let mut documents = Vec::new();
    while !decoded.is_empty() {
        if decoded.len() < 4 {
            return Err(BundleError::Format("truncated length prefix"));
        }
        let (length, rest) = decoded.split_at(4);
        let length = u32::from_le_bytes([length[0], length[1], length[2], length[3]]) as usize;
        if rest.len() < length {
            return Err(BundleError::Format("truncated document"));
        }
        let (document, rest) = rest.split_at(length);
        documents.push(
            std::str::from_utf8(document).map_err(|_| BundleError::Format("non utf-8 document"))?,
        );
        decoded = rest;
    }
    Ok(documents)
}

impl Catalog {
    /// Build a catalog from a compressed dataset bundle.
    pub fn from_bundle(bundle: &[u8]) -> Result<Self, BundleError> {
        let decoded = decode_bundle(bundle)?;
        let documents = split_bundle(&decoded)?;
        Self::from_json(documents).map_err(BundleError::Json)
    }
}
