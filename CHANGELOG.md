# Changelog

Version scheme: **`1.<YYMM>.<patch>`** (hybrid CalVer, semver-compatible)

- `1` — generation of the bundle format / public API. A breaking change is `2.0.0`.
- `<YYMM>` — month the release is cut (`2611` = November 2026); the data sync happens
  as part of the release, so this is the snapshot month in the normal case.
- `<patch>` — release ordinal within that month.

Consumers depend on `^1`, so a codelist refresh (new data, same API) reaches every
release line without a dependency bump.

## 1.2611.0 — 2026-11-06

- Published through **trusted publishing only** (crates.io OIDC + npm OIDC, no
  long-lived tokens).
- Fixed the npm package: the `labels/` variant was missing from `files` and from
  the `exports` map, so `xoev-codelists/labels/<slug>.json` was not importable.
- Post-release verification job: installs the published crates and npm package and
  asserts the version-aware `contains()` plus all three dataset variants.

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
