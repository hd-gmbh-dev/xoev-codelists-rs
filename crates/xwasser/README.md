# xoev-codelists-xwasser

Every published version of the codelists below, addressable as **URN + version**.

Part of [`xoev-codelists`](https://crates.io/crates/xoev-codelists); see the [repository](https://github.com/hd-gmbh-dev/xoev-codelists-rs) for the data model and the other publishers.

## Usage

```toml
[dependencies]
xoev-codelists-xwasser = { version = "1", default-features = false, features = ["codes"] }
```

`codes` validates a transmitted code and is the smallest bundle; `labels` adds the display column, `full` all columns.

```rust
use xoev_codelists_xwasser::catalog;

let catalog = catalog();
assert!(catalog.contains("<urn>", "<version>", "<code>").is_some());
```

## Codelists

| codelist | current version | versions | codes (current) |
|---|---|---|---|
| `urn:xoev-de:xwasser:codeliste:art-entnahmearmatur` | `1` | 1 | 15 |
| `urn:xoev-de:xwasser:codeliste:art-objekt` | `3` | 3 | 62 |
| `urn:xoev-de:xwasser:codeliste:art-probennahmestelle` | `2` | 2 | 31 |
| `urn:xoev-de:xwasser:codeliste:aufbereitungsstoffe-desinfektionsverfahren` | `3` | 3 | 105 |
| `urn:xoev-de:xwasser:codeliste:desinfektionsart` | `2` | 2 | 10 |
| `urn:xoev-de:xwasser:codeliste:format-alternative-id` | `1` | 1 | 7 |
| `urn:xoev-de:xwasser:codeliste:massnahmen` | `2` | 2 | 21 |
| `urn:xoev-de:xwasser:codeliste:medium` | `3` | 3 | 11 |
| `urn:xoev-de:xwasser:codeliste:parameterauspraegung` | `5` | 5 | 208 |
| `urn:xoev-de:xwasser:codeliste:probenentnahmegeraet` | `1` | 1 | 5 |
| `urn:xoev-de:xwasser:codeliste:probengefaess` | `1` | 1 | 5 |
| `urn:xoev-de:xwasser:codeliste:probennahmeverfahren` | `4` | 4 | 11 |
| `urn:xoev-de:xwasser:codeliste:shapth-parameter` | `5` | 5 | 2020 |
| `urn:xoev-de:xwasser:codeliste:shapth-parameter-einheit` | `2` | 2 | 47 |
| `urn:xoev-de:xwasser:codeliste:unterkategorie-probennahmestelle` | `1` | 1 | 16 |
| `urn:xoev-de:xwasser:codeliste:untersuchung-anlass` | `3` | 3 | 21 |
| `urn:xoev-de:xwasser:codeliste:untersuchungsverfahren` | `3` | 3 | 235 |
