# xoev-codelists-dsmeld

Every published version of the codelists below, addressable as **URN + version**.

Part of [`xoev-codelists`](https://crates.io/crates/xoev-codelists); see the [repository](https://github.com/hd-gmbh-dev/xoev-codelists-rs) for the data model and the other publishers.

## Usage

```toml
[dependencies]
xoev-codelists-dsmeld = { version = "1", default-features = false, features = ["codes"] }
```

`codes` validates a transmitted code and is the smallest bundle; `labels` adds the display column, `full` all columns.

```rust
use xoev_codelists_dsmeld::catalog;

let catalog = catalog();
assert!(catalog.contains("<urn>", "<version>", "<code>").is_some());
```

## Codelists

| codelist | current version | versions | codes (current) |
|---|---|---|---|
| `urn:de:dsmeld:schluesseltabelle:auskunftssperre` | `9` | 7 | 7 |
| `urn:de:dsmeld:schluesseltabelle:familienstand` | `2` | 2 | 10 |
| `urn:de:dsmeld:schluesseltabelle:familienstand.beendigungsgrund` | `3` | 3 | 7 |
| `urn:de:dsmeld:schluesseltabelle:pass.und.ausweisdokumente` | `5` | 4 | 15 |
