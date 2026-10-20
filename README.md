# xoev-codelists

Versioned codelists referenced by XÖV standards, addressable as **URN + version**.

For XÖV **code type 3** (`http://xoev.de/schemata/code/1_0`, datatype `Code`) the
codelist URI is fixed by the standard while the version is chosen by the sender
(`listVersionID`), so a receiver must be able to resolve **any published
version**, not only the one that was current at release time.

## Crates

| crate | publisher namespace |
|---|---|
| `xoev-codelists` | core model (`CodelistId` = URN + version) |
| `xoev-codelists-xwasser` | `urn:xoev-de:xwasser:codeliste:` |
| `xoev-codelists-destatis` | `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:` |
| `xoev-codelists-xunternehmen` | `urn:xoev-de:xunternehmen:codeliste:` |
| `xoev-codelists-dsmeld` | `urn:de:dsmeld:schluesseltabelle:` |
| `xoev-codelists-xauslaender` | `urn:xoev-de:xauslaender:codeliste:` |
| `xoev-codelists-bmi-bit` | `urn:xoev-de:bund:bmi:bit:codeliste:` |

One crate per **publisher dataset**, not per codelist: codelist versions tick
independently, but a single codelist does not justify its own crate/publish
cycle. Per-codelist granularity is a cargo feature of the publisher crate.

## npm

`xoev-codelists` — a single package with an index keyed by `{urn, version}`.

## Usage

```rust
use xoev_codelists_xwasser::catalog;

let catalog = catalog();
let urn = "urn:xoev-de:xwasser:codeliste:untersuchungsverfahren";

// newest published version
let current = catalog.current_version(urn).unwrap();

// a version chosen by the sender of a message (XÖV code type 3)
let snapshot = catalog.resolve(urn, "1").unwrap();
let label = snapshot.text("1010", "Untersuchungsverfahren").unwrap();

// validation against the version the document declares
assert_eq!(catalog.contains(urn, "1", "1010"), Some(true));
assert_eq!(catalog.contains(urn, "1", "9999"), Some(false));
```

## Data

`crates/<publisher>/data/datasets.zst` is one **compressed bundle per publisher**:
every codelist with its newest version as a full snapshot and **reverse deltas**
towards older versions. Bundling compressed is what ends up inside binaries and
wasm builds; the individual JSON documents are regenerated locally (gitignored)
because they are generated data.

### Bundles: full vs codes-only

Each publisher crate embeds one bundle, selected by cargo feature:

| features | bundle | contains | use |
|---|---|---|---|
| `full` (default) | `datasets.zst` | all columns per version | labels, limits, full validation |
| `codes` (with `default-features = false`) | `codes.zst` | the key column only | validating a transmitted code |

Code validation is the smallest useful artifact: the whole code-only bundle for
**all 27 codelists and 272 versions is 56 KiB** (vs. 412 KiB full). Note that
hashing the names instead of shipping them is *not* smaller — German place names
compress better than random digests.

### Sizes (all publishers, 2026-10)

| publisher | codelists | versions | raw JSON | bundle (zstd -19) |
|---|---|---|---|---|
| `xwasser` | 16 | 41 | 697 KiB | **109 KiB** |
| `destatis` | 6 | 164 | 1881 KiB | **270 KiB** |
| `xunternehmen` | 2 | 7 | 86 KiB | 10 KiB |
| `xauslaender` | 1 | 9 | 51 KiB | 18 KiB |
| `bmi-bit` | 1 | 47 | 19 KiB | 3.6 KiB |
| `dsmeld` | 1 | 4 | 4 KiB | 0.8 KiB |
| **total** | **27** | **272** | **2.7 MiB** | **412 KiB** |
| **code-only bundles** | **27** | **272** | 55 KiB | **56 KiB** |

### Why deltas

A codelist version is almost never a new list — measured on real pairs, a version
step touches 0.05 %–15 % of the rows (an AGS step changes 16 of 11 227 rows),
while one snapshot per version would be ~40 MiB for AGS alone.

Three rules keep a chain small:

- **Column aliasing** — when a column is renamed between versions (`SHATPH-Parameter-ID`
  → `SHAPTH-Parameter-ID`), the delta records one rename instead of a changed cell
  in every row. Renames are detected by comparing values across shared keys
  (≥ 90 % equal).
- **Resync fallback** — a step that would not be smaller than the snapshot it
  rebuilds (e.g. a version that renumbers all keys) is stored as a snapshot.
- **Canonical row order** — rows are sorted by key on load, because some
  published versions are not sorted. Duplicate keys (Destatis `staat` 2011 has
  two rows for key `168`) are kept and identified by key plus occurrence.

### Regenerate

```sh
cargo run -p xtask -- sync --publisher xwasser
```

The command downloads every published version from XRepository (with a disk cache
under `target/xrepository-cache`), reports the size of each step, rebuilds each
version from the chain and aborts if a reconstruction differs from the original.
Steps above 90 % of the snapshot they rebuild are treated as suspect
(`ALLOW_LARGE_DELTAS=1` accepts them), a step above 25 % and 8 KiB is annotated
as content churn. Versions that XRepository lists but no longer serves (404) are
reported and skipped. `--since <date>` limits the history that is kept.

## Status

`0.1.0-alpha.x` reserves the crate and package names; the datasets (base version
plus reverse deltas per older version) follow.

## License

MIT