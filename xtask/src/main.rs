//! Codelist sync: fetch every published version of a publisher's codelists from
//! XRepository and write one dataset per codelist (newest version as snapshot,
//! older versions as reverse deltas).

mod codelists;

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use serde::de::DeserializeOwned;
use xoev_codelists::{Dataset, Snapshot, Step};
use xrepository_rs::model::xcodelist::XCodelist;
use xrepository_rs::model::xcodelist_data::XCodelistData;
use xrepository_rs::model::xcodelist_with_version::XCodelistWithVersion;

const XREPOSITORY: &str = "https://www.xrepository.de/api/xrepository";

/// Versions XRepository lists but does not serve.
static SKIPPED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// XRepository access with a disk cache: a full sync downloads hundreds of
/// megabytes, so a re-run must not repeat that.
struct Source {
    http: reqwest::Client,
    cache: PathBuf,
    downloaded: usize,
    downloaded_bytes: usize,
    cached: usize,
}

impl Source {
    fn new(cache: PathBuf) -> Self {
        Self {
            http: reqwest::Client::new(),
            cache,
            downloaded: 0,
            downloaded_bytes: 0,
            cached: 0,
        }
    }

    /// Fetch and parse, `Ok(None)` when the resource does not exist (404).
    async fn try_json<T: DeserializeOwned>(&mut self, path: &str) -> Result<Option<T>> {
        let file = self.cache.join(format!("{}.json", fingerprint(path)));
        if let Ok(bytes) = tokio::fs::read(&file).await {
            self.cached += 1;
            return serde_json::from_slice(&bytes)
                .map(Some)
                .with_context(|| format!("cached {path} is not valid json"));
        }

        let response = self
            .http
            .get(format!("{XREPOSITORY}/{path}"))
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .with_context(|| format!("GET {path}"))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response
            .error_for_status()
            .with_context(|| format!("GET {path}"))?;
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("read {path}"))?;
        tokio::fs::create_dir_all(&self.cache).await.ok();
        tokio::fs::write(&file, &bytes).await.ok();
        self.downloaded += 1;
        self.downloaded_bytes += bytes.len();
        serde_json::from_slice(&bytes)
            .map(Some)
            .with_context(|| format!("parse {path}"))
    }

    async fn json<T: DeserializeOwned>(&mut self, path: &str) -> Result<T> {
        self.try_json(path)
            .await?
            .ok_or_else(|| anyhow!("{path}: 404 not found"))
    }
}

fn fingerprint(path: &str) -> String {
    // no hashing dependency for a cache key: the path is URL-safe already
    path.replace(['/', ':', '.', ' '], "_")
}

/// What a bundle carries. `Codes` keeps only the key column, which is all a
/// receiver needs to validate a transmitted code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Variant {
    Full,
    /// Key column plus the label column: enough for a dropdown.
    Labels,
    /// Key column only: enough to validate a transmitted code.
    Codes,
}

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Download all versions and write `crates/<publisher>/data/<slug>.json`.
    Sync {
        /// Publisher crate name, e.g. `xwasser`.
        #[arg(long)]
        publisher: String,
        /// Only this codelist URN (default: all of the publisher).
        #[arg(long)]
        urn: Option<String>,
        /// What to bundle: the full rows, or only the codes (validation).
        #[arg(long, value_enum, default_value_t = Variant::Full)]
        variant: Variant,
        /// Skip versions that were valid before this date (ISO, e.g. 2024-01-01).
        #[arg(long)]
        since: Option<String>,
        /// Directory to write the datasets to.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Write the npm package from the committed bundles (no network needed).
    Export {
        /// Directory of the npm package, e.g. `npm`.
        #[arg(long, default_value = "npm")]
        npm: PathBuf,
    },
    /// List the codelists of a publisher and their published versions.
    List {
        #[arg(long)]
        publisher: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let Args { command } = Args::parse();
    let mut source = Source::new(PathBuf::from("target/xrepository-cache"));

    match command {
        Command::Sync {
            publisher,
            urn,
            out,
            since,
            variant,
        } => {
            let publisher = codelists::publisher(&publisher)?;
            let out = out.unwrap_or_else(|| {
                PathBuf::from("crates")
                    .join(publisher.crate_name)
                    .join("data")
            });
            tokio::fs::create_dir_all(&out)
                .await
                .with_context(|| format!("create {}", out.display()))?;

            let mut bundle_documents: Vec<(String, Vec<u8>)> = Vec::new();
            let bundle_name = match variant {
                Variant::Full => "datasets.zst",
                Variant::Labels => "labels.zst",
                Variant::Codes => "codes.zst",
            };
            for codelist in publisher.codelists {
                if urn.as_deref().is_some_and(|u| u != codelist.urn) {
                    continue;
                }
                let mut snapshots =
                    fetch_versions(&mut source, codelist.urn, since.as_deref()).await?;
                if !codelist.versioned && snapshots.len() > 1 {
                    // code type 1/2: the standard fixes the version, so only the
                    // pinned one is kept
                    snapshots = snapshots.split_off(snapshots.len() - 1);
                }
                let snapshots = match variant {
                    Variant::Full => snapshots.clone(),
                    Variant::Labels => snapshots
                        .iter()
                        .map(|snapshot| projected(snapshot, codelists::label_column(codelist.urn)))
                        .collect(),
                    Variant::Codes => snapshots.iter().map(codes_only).collect(),
                };
                let dataset = Dataset::from_snapshots(snapshots.clone())
                    .with_context(|| format!("build dataset for {}", codelist.urn))?;

                // Every published version must be reproducible from base + deltas.
                for snapshot in &snapshots {
                    let snapshot = &snapshot.clone().canonicalized()?;
                    let rebuilt = dataset
                        .resolve(&snapshot.version)
                        .ok_or_else(|| anyhow!("version {} not resolvable", snapshot.version))?;
                    if rebuilt != *snapshot {
                        bail!(
                            "{}: reconstruction of version {} differs from the original",
                            codelist.urn,
                            snapshot.version
                        );
                    }
                }

                // Keep every step small: print what each one costs and refuse to
                // silently store a delta that is not worth keeping.

                for (index, step) in dataset.steps.iter().enumerate() {
                    let older = &snapshots[snapshots.len() - 2 - index]
                        .clone()
                        .canonicalized()?;
                    let step_size = serde_json::to_vec(step)?.len();
                    let version_size = serde_json::to_vec(older)?.len().max(1);
                    let ratio = step_size as f64 / version_size as f64;
                    let (kind, note) = match step {
                        Step::Snapshot(_) => ("resync", ""),
                        Step::Delta(_delta) => {
                            // deltas are capped at half the snapshot by
                            // construction, so anything above 25% is worth a look
                            let note = if ratio > 0.25 {
                                " <-- mostly rewritten"
                            } else {
                                ""
                            };

                            ("delta", note)
                        }
                    };
                    println!(
                        "  {:<3} -> {:<6} {:>7} B of {:>8} B ({:>5.1}%){}",
                        dataset.base.version,
                        step.version(),
                        step_size,
                        version_size,
                        ratio * 100.0,
                        note
                    );
                    let _ = kind;
                }

                let _ = &mut source;
                let slug = codelists::slug(codelist.urn);
                let path = out.join(format!("{slug}.json"));
                let document = serde_json::to_vec(&dataset)?;
                bundle_documents.push((codelist.urn.to_string(), document.clone()));
                tokio::fs::write(&path, document)
                    .await
                    .with_context(|| format!("write {}", path.display()))?;
                println!(
                    "{:<70} {:<8} versions: {} ({})",
                    codelist.urn,
                    slug,
                    dataset.versions().count(),
                    dataset.versions().collect::<Vec<_>>().join(" < ")
                );
            }
            // One compressed bundle per publisher: this is what the crates
            // embed, the individual JSON files stay out of version control.
            bundle_documents.sort_by(|a, b| a.0.cmp(&b.0));
            let mut raw = Vec::new();
            for (_, document) in &bundle_documents {
                raw.extend_from_slice(&(document.len() as u32).to_le_bytes());
                raw.extend_from_slice(document);
            }
            let compressed = zstd::encode_all(raw.as_slice(), 19)?;
            let bundle_path = out.join(bundle_name);
            tokio::fs::write(&bundle_path, &compressed).await?;
            println!(
                "\nbundle {}: {} codelists, {} KiB raw -> {} KiB compressed",
                bundle_path.display(),
                bundle_documents.len(),
                raw.len() / 1024,
                compressed.len() / 1024
            );

            let skipped = SKIPPED.lock().expect("skip list");
            if !skipped.is_empty() {
                println!(
                    "\n{} version(s) listed by XRepository but not served (skipped):",
                    skipped.len()
                );
                for version in skipped.iter() {
                    println!("  {version}");
                }
            }
            println!(
                "\nhttp: {} downloaded ({} MiB), {} cached",
                source.downloaded,
                source.downloaded_bytes / (1024 * 1024),
                source.cached
            );
        }
        Command::Export { npm } => {
            export_npm(&npm).await?;
        }
        Command::List { publisher } => {
            let publisher = codelists::publisher(&publisher)?;
            for codelist in publisher.codelists {
                let versions = source
                    .json::<XCodelist>(&format!("{}/metadaten", codelist.urn))
                    .await
                    .with_context(|| format!("metadata for {}", codelist.urn))?
                    .alle_versions_kennungen
                    .unwrap_or_default();
                println!("{} ({} versions)", codelist.urn, versions.len());
            }
        }
    }

    Ok(())
}

/// Fetch all published versions of `urn`, ordered oldest → newest.
async fn fetch_versions(
    source: &mut Source,
    urn: &str,
    since: Option<&str>,
) -> Result<Vec<Snapshot>> {
    let version_urns = source
        .json::<XCodelist>(&format!("{urn}/metadaten"))
        .await
        .with_context(|| format!("metadata for {urn}"))?
        .alle_versions_kennungen
        .unwrap_or_default();

    let mut snapshots = Vec::with_capacity(version_urns.len());
    for version_urn in version_urns {
        // XRepository lists versions it no longer serves (withdrawn or never
        // published); they are reported and skipped.
        let Some(meta) = source
            .try_json::<XCodelistWithVersion>(&format!("{version_urn}/metadaten"))
            .await
            .with_context(|| format!("metadata for {version_urn}"))?
        else {
            SKIPPED.lock().expect("skip list").push(version_urn.clone());
            continue;
        };
        let path = meta
            .download_urn()
            .ok_or_else(|| anyhow!("{version_urn}: no JSON document published"))?
            .to_string();
        if let Some(since) = since
            && meta
                .datum_gueltigkeit_ab
                .as_deref()
                .is_some_and(|from| from < since)
        {
            continue;
        }
        let Some(data) = source
            .try_json::<XCodelistData>(&path)
            .await
            .with_context(|| format!("download {path}"))?
        else {
            SKIPPED.lock().expect("skip list").push(version_urn);
            continue;
        };
        snapshots.push(snapshot(&version_urn, data)?);
    }

    // `alle_versions_kennungen` is not ordered, and string ordering breaks on
    // `_10` vs `_2`, so order by the published validity start.
    snapshots.sort_by(|a, b| {
        a.valid_from
            .cmp(&b.valid_from)
            .then_with(|| a.version.cmp(&b.version))
    });
    Ok(snapshots)
}

/// Reduce a snapshot to the key column plus the label column.
fn projected(snapshot: &Snapshot, label_column: Option<&str>) -> Snapshot {
    let key_index = snapshot.key_index;
    let key = snapshot.columns[key_index].clone();
    let label_index = label_column
        .and_then(|column| snapshot.columns.iter().position(|name| name == column))
        .unwrap_or(if snapshot.columns.len() > 1 { 1 } else { key_index });
    if label_index == key_index {
        return codes_only(snapshot);
    }
    Snapshot {
        urn: snapshot.urn.clone(),
        version: snapshot.version.clone(),
        valid_from: snapshot.valid_from.clone(),
        columns: vec![key.clone(), snapshot.columns[label_index].clone()],
        key_index: 0,
        rows: snapshot
            .rows
            .iter()
            .map(|row| vec![row[key_index].clone(), row[label_index].clone()])
            .collect(),
    }
}

/// Reduce a snapshot to its key column: what code validation needs.
fn codes_only(snapshot: &Snapshot) -> Snapshot {
    Snapshot {
        urn: snapshot.urn.clone(),
        version: snapshot.version.clone(),
        valid_from: snapshot.valid_from.clone(),
        columns: vec![snapshot.columns[snapshot.key_index].clone()],
        key_index: 0,
        rows: snapshot
            .rows
            .iter()
            .map(|row| vec![row[snapshot.key_index].clone()])
            .collect(),
    }
}

fn snapshot(version_urn: &str, data: XCodelistData) -> Result<Snapshot> {
    let meta = data
        .metadaten
        .ok_or_else(|| anyhow!("{version_urn}: no metadata in codelist data"))?;
    let urn = meta
        .kennung_inhalt
        .ok_or_else(|| anyhow!("{version_urn}: no canonical urn in codelist data"))?;
    let version = meta
        .version
        .ok_or_else(|| anyhow!("{version_urn}: no version in codelist data"))?;
    let spalten = data.spalten.unwrap_or_default();
    let key_index = spalten
        .iter()
        .position(|column| column.code_spalte == Some(true))
        .unwrap_or(0);
    let columns: Vec<String> = spalten
        .into_iter()
        .enumerate()
        .map(|(idx, column)| {
            column
                .spaltenname_technisch
                .unwrap_or_else(|| format!("Spalte{}", idx + 1))
        })
        .collect();
    if columns.is_empty() {
        bail!("{version_urn}: codelist has no columns");
    }

    let rows = (data.daten.unwrap_or_default())
        .into_iter()
        .map(|row| {
            let mut values: Vec<String> = row
                .into_iter()
                .map(|value| match value {
                    serde_json::Value::Null => String::new(),
                    serde_json::Value::String(text) => text,
                    other => other.to_string(),
                })
                .collect();
            values.resize(columns.len(), String::new());
            values
        })
        .filter(|row| !row[key_index].is_empty())
        .collect();

    Ok(Snapshot {
        urn: urn.into(),
        version: version.into(),
        valid_from: meta.gueltig_ab,
        columns,
        key_index,
        rows,
    })
}

/// Write the npm package: an index plus one file per codelist (full rows and
/// codes only), so a JavaScript consumer can lazily import just the codelist it
/// needs instead of shipping a bundle.
async fn export_npm(npm: &std::path::Path) -> Result<()> {
    let codelists_dir = npm.join("codelists");
    let labels_dir = npm.join("labels");
    let codes_dir = npm.join("codes");
    tokio::fs::create_dir_all(&codelists_dir).await?;
    tokio::fs::create_dir_all(&labels_dir).await?;
    tokio::fs::create_dir_all(&codes_dir).await?;

    let mut index = Vec::new();
    let mut publishers: Vec<PathBuf> = glob_bundles()?;
    publishers.sort();

    for bundle in publishers {
        let publisher = bundle
            .parent()
            .and_then(|data| data.parent())
            .and_then(|crates| crates.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();

        for (file, variant) in [
            (Some(bundle.clone()), "datasets"),
            (sibling(&bundle, "labels.zst"), "labels"),
            (sibling(&bundle, "codes.zst"), "codes"),
        ] {
            let Some(file) = file else { continue };
            if !file.exists() {
                continue;
            }
            let bundle = tokio::fs::read(&file).await?;
            let decoded = xoev_codelists::decode_bundle(&bundle)
                .map_err(|err| anyhow!("{}: {err}", file.display()))?;
            let documents = xoev_codelists::split_bundle(&decoded)
                .map_err(|err| anyhow!("{}: {err}", file.display()))?;

            for document in documents {
                let dataset: Dataset = serde_json::from_str(document)?;
                let slug = codelists::slug(&dataset.urn);
                let dir = match variant {
                    "datasets" => &codelists_dir,
                    "labels" => &labels_dir,
                    _ => &codes_dir,
                };
                let path = dir.join(format!("{slug}.json"));
                tokio::fs::write(&path, document.as_bytes()).await?;

                if variant == "datasets" {
                    index.push(serde_json::json!({
                        "urn": dataset.urn,
                        "publisher": publisher,
                        "versions": dataset.versions().collect::<Vec<_>>(),
                        "current": dataset.current_version(),
                        "rows": format!("codelists/{slug}.json"),
                        "labels": format!("labels/{slug}.json"),
                        "codes": format!("codes/{slug}.json"),
                    }));
                }
            }
        }
    }

    index.sort_by(|a, b| a["urn"].as_str().cmp(&b["urn"].as_str()));
    let index = serde_json::json!({
        "schema": 1,
        "codelists": index,
    });
    tokio::fs::write(npm.join("index.json"), serde_json::to_vec_pretty(&index)?).await?;
    println!(
        "npm package {}: {} codelists",
        npm.display(),
        index["codelists"].as_array().map(Vec::len).unwrap_or(0)
    );
    Ok(())
}

fn glob_bundles() -> Result<Vec<PathBuf>> {
    let mut bundles = Vec::new();
    for entry in std::fs::read_dir("crates")? {
        let path = entry?.path().join("data").join("datasets.zst");
        if path.exists() {
            bundles.push(path);
        }
    }
    Ok(bundles)
}

fn sibling(bundle: &std::path::Path, name: &str) -> Option<PathBuf> {
    bundle.parent().map(|parent| parent.join(name))
}
