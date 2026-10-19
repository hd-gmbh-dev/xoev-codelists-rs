//! Codelist sync: fetch every published version of a publisher's codelists from
//! XRepository and write one dataset per codelist (newest version as snapshot,
//! older versions as reverse deltas).

mod codelists;

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use xoev_codelists::{Dataset, Snapshot, Step};
use xrepository_rs::client::Client;
use xrepository_rs::model::xcodelist_data::XCodelistData;

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
        /// Directory to write the datasets to.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Report a delta as suspect when it is larger than this share of the
        /// version it reconstructs (such a step usually means an unaliased
        /// column rename or a changed key scheme).
        #[arg(long, default_value_t = 0.9)]
        max_delta_ratio: f64,
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
    let client = Client::new();

    match command {
        Command::Sync {
            publisher,
            urn,
            out,
            max_delta_ratio,
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

            for codelist in publisher.codelists {
                if urn.as_deref().is_some_and(|u| u != codelist.urn) {
                    continue;
                }
                let mut snapshots = fetch_versions(&client, codelist.urn).await?;
                if !codelist.versioned && snapshots.len() > 1 {
                    // code type 1/2: the standard fixes the version, so only the
                    // pinned one is kept
                    snapshots = snapshots.split_off(snapshots.len() - 1);
                }
                let dataset = Dataset::from_snapshots(snapshots.clone())
                    .with_context(|| format!("build dataset for {}", codelist.urn))?;

                // Every published version must be reproducible from base + deltas.
                for snapshot in &snapshots {
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
                let mut suspect: Vec<String> = Vec::new();
                for (index, step) in dataset.steps.iter().enumerate() {
                    let older = &snapshots[snapshots.len() - 2 - index];
                    let step_size = serde_json::to_vec(step)?.len();
                    let version_size = serde_json::to_vec(older)?.len().max(1);
                    let ratio = step_size as f64 / version_size as f64;
                    let (kind, note) = match step {
                        Step::Snapshot(_) => ("resync", ""),
                        Step::Delta(delta) => {
                            let note = if ratio > max_delta_ratio.max(0.5) {
                                suspect.push(format!("{} ({:.0}%)", delta.to, ratio * 100.0));
                                " <-- check"
                            } else if ratio > 0.25 && step_size > 8 * 1024 {
                                " (content churn)"
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

                let slug = codelists::slug(codelist.urn);
                let path = out.join(format!("{slug}.json"));
                tokio::fs::write(&path, serde_json::to_vec_pretty(&dataset)?)
                    .await
                    .with_context(|| format!("write {}", path.display()))?;
                println!(
                    "{:<70} {:<8} versions: {} ({})",
                    codelist.urn,
                    slug,
                    dataset.versions().count(),
                    dataset.versions().collect::<Vec<_>>().join(" < ")
                );
                if !suspect.is_empty() {
                    println!(
                        "  WARNING: delta near snapshot size for {}: {}",
                        codelist.urn,
                        suspect.join(", ")
                    );
                    if std::env::var("ALLOW_LARGE_DELTAS").is_err() {
                        bail!(
                            "{}: {} step(s) exceed the delta budget (set ALLOW_LARGE_DELTAS=1 to accept)",
                            codelist.urn,
                            suspect.len()
                        );
                    }
                }
            }
        }
        Command::List { publisher } => {
            let publisher = codelists::publisher(&publisher)?;
            for codelist in publisher.codelists {
                let versions = client
                    .fetch_xcodelist(codelist.urn)
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
async fn fetch_versions(client: &Client, urn: &str) -> Result<Vec<Snapshot>> {
    let version_urns = client
        .fetch_xcodelist(urn)
        .await
        .with_context(|| format!("metadata for {urn}"))?
        .alle_versions_kennungen
        .unwrap_or_default();

    let mut snapshots = Vec::with_capacity(version_urns.len());
    for version_urn in version_urns {
        let meta = client
            .fetch_xcodelist_with_version(&version_urn)
            .await
            .with_context(|| format!("metadata for {version_urn}"))?;
        let path = meta
            .download_urn()
            .ok_or_else(|| anyhow!("{version_urn}: no JSON document published"))?
            .to_string();
        let data = client
            .download(&path)
            .await
            .with_context(|| format!("download {path}"))?;
        snapshots.push(snapshot(&version_urn, data)?);
    }

    // `alle_versions_kennungen` is not ordered, and string ordering breaks on
    // `_10` vs `_2`, so order by the published validity start.
    snapshots.sort_by(|a, b| {
        a.valid_from
            .cmp(&b.valid_from)
            .then_with(|| a.version.cmp(&b.version))
    });
    if let Some(last) = snapshots.len().checked_sub(1) {
        for pair in snapshots[..last].windows(2) {
            if pair[0].valid_from == pair[1].valid_from && pair[0].version > pair[1].version {
                bail!("{urn}: cannot order versions {pair:?}");
            }
        }
    }
    Ok(snapshots)
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
