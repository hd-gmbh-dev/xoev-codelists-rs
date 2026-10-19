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

`crates/<publisher>/data/<slug>.json` holds one dataset per codelist: the newest
version as a full snapshot plus **reverse deltas** towards older versions. Deltas
because a codelist version is almost never a new list — measured on real pairs, a
version step touches 0.05 %–15 % of the rows (an AGS step changes 16 of 11 227
rows), while a full snapshot per version would be ~300 MB for AGS alone.

Two rules keep a chain small:

- **Column aliasing** — when a column is renamed between versions (`SHATPH-Parameter-ID`
  → `SHAPTH-Parameter-ID`), the delta records one rename instead of a changed cell
  in every row. Renames are detected by comparing values across shared keys
  (≥ 90 % equal).
- **Resync fallback** — a step that would not be smaller than the snapshot it
  rebuilds (e.g. a version that renumbers all keys) is stored as a snapshot.

Regenerate with

```sh
cargo run -p xtask -- sync --publisher xwasser
```

The command downloads every published version from XRepository, reports the size
of each step, rebuilds each version from the chain and aborts if a reconstruction
differs from the original. Steps above 90 % of the snapshot they rebuild are
treated as suspect; a step above 25 % and 8 KB is annotated as content churn and
worth a look (`ALLOW_LARGE_DELTAS=1` accepts suspect steps).

## Status

`0.1.0-alpha.x` reserves the crate and package names; the datasets (base version
plus reverse deltas per older version) follow.

## License

MIT