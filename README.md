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

## Status

`0.1.0-alpha.x` reserves the crate and package names; the datasets (base version
plus reverse deltas per older version) follow.

## License

MIT