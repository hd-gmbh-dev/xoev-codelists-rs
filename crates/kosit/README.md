# xoev-codelists-kosit

Every published version of the codelists below, addressable as **URN + version**.

Part of [`xoev-codelists`](https://crates.io/crates/xoev-codelists); see the [repository](https://github.com/hd-gmbh-dev/xoev-codelists-rs) for the data model and the other publishers.

## Usage

```toml
[dependencies]
xoev-codelists-kosit = { version = "1", default-features = false, features = ["codes"] }
```

`codes` validates a transmitted code and is the smallest bundle; `labels` adds the display column, `full` all columns.

```rust
use xoev_codelists_kosit::catalog;

let catalog = catalog();
assert!(catalog.contains("<urn>", "<version>", "<code>").is_some());
```

## Codelists

| codelist | current version | versions | codes (current) |
|---|---|---|---|
| `urn:xoev-de:kosit:codeliste:currency-codes` | `4` | 4 | 178 |
