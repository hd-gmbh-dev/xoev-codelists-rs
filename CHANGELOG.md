# Changelog

Version scheme: **`1.<YYMM>.<patch>`** (hybrid CalVer, semver-compatible)

- `1` — generation of the bundle format / public API. A breaking change is `2.0.0`.
- `<YYMM>` — month of the data snapshot (`2610` = October 2026).
- `<patch>` — release ordinal within that month.

Consumers depend on `^1`, so a codelist refresh (new data, same API) reaches every
release line without a dependency bump.

## 1.2610.0 — 2026-10-22

First stable release.

- `xoev-codelists`: `Snapshot` / `Delta` / `Step` / `Dataset` / `Catalog`,
  compressed bundle decoding (`decode_bundle`, `split_bundle`, `Catalog::from_bundle`).
- Six publisher crates (`xwasser`, `destatis`, `xunternehmen`, `dsmeld`,
  `xauslaender`, `bmi-bit`): 28 codelists, 273 versions, three bundle variants —
  `full` (all columns), `labels` (key + label), `codes` (key only) — selected by
  cargo feature.
- Data: every published version of the 27 type-3 codelists of XWasser 1.0.1 plus
  `format-alternative-id` (type 3 in the 0.9.x lines, legacy entry).
