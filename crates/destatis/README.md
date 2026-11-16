# xoev-codelists-destatis

Every published version of the codelists below, addressable as **URN + version**.

Part of [`xoev-codelists`](https://crates.io/crates/xoev-codelists); see the [repository](https://github.com/hd-gmbh-dev/xoev-codelists-rs) for the data model and the other publishers.

## Usage

```toml
[dependencies]
xoev-codelists-destatis = { version = "1", default-features = false, features = ["codes"] }
```

`codes` validates a transmitted code and is the smallest bundle; `labels` adds the display column, `full` all columns.

```rust
use xoev_codelists_destatis::catalog;

let catalog = catalog();
assert!(catalog.contains("<urn>", "<version>", "<code>").is_some());
```

## Codelists

| codelist | current version | versions | codes (current) |
|---|---|---|---|
| `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:ags` | `2026-09-30` | 106 | 11227 |
| `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:bezirk` | `2019-07-31` | 6 | 36 |
| `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:bundesland` | `2010-04-01` | 1 | 16 |
| `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:kreis` | `2026-01-31` | 16 | 518 |
| `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:rs` | `2026-09-30` | 1 | 11227 |
| `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:staat` | `2026-08-05` | 18 | 197 |
| `urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:staatsangehoerigkeit` | `2026-08-05` | 17 | 211 |
