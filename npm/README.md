# xoev-codelists

Versioned codelists referenced by XÖV standards, addressable as **URN + version**.

For XÖV **code type 3** (`http://xoev.de/schemata/code/1_0`, datatype `Code`) the
codelist URI is fixed by the standard while the version is chosen by the sender
(`listVersionID`), so a receiver must be able to resolve **any published
version**, not only the one that was current at release time.

27 codelists, 272 versions — XWasser, Destatis, XUnternehmen, DSMeld,
XAusländer and BMI/BIT.

## Layout

```
index.json                 all codelists with their published versions
codelists/<slug>.json      full dataset: all rows, columns and labels per version
codes/<slug>.json          code-only dataset: just the valid codes per version
```

A dataset is one snapshot of the newest version plus reverse deltas towards older
versions (`{"base": …, "steps": [{"delta": …} | {"snapshot": …}]}`) — small
enough to ship, exact enough to rebuild every version.

## Usage

```js
import index from 'xoev-codelists/index.json';

const urn = 'urn:xoev-de:xwasser:codeliste:untersuchungsverfahren';
const entry = index.codelists.find((codelist) => codelist.urn === urn);
// entry.current -> newest published version
// entry.versions -> all published versions, newest first
// entry.codes    -> path to the code-only dataset
// entry.labels   -> path to the full dataset

// only load what you need
const dataset = (await import(`xoev-codelists/${entry.labels}`)).default;
```

Rebuild a version by folding the deltas, or precompute the few versions the
application supports:

```js
function resolve(dataset, version) {
    let current = structuredClone(dataset.base);
    if (current.version === version) return current;
    for (const step of dataset.steps ?? []) {
        current = step.snapshot ? structuredClone(step.snapshot) : apply(current, step.delta);
        if (current.version === version) return current;
    }
    return undefined;
}
```

`apply(snapshot, delta)` removes `delta.removed_rows`, restores
`delta.restored_cells` (`{key, column, value}`, column as index into
`delta.columns`), maps the remaining columns (`delta.renamed_columns` allows for
renamed columns) and appends `delta.added_rows`. The Rust reference
implementation is `xoev_codelists::apply`.

For validation only, `codes/<slug>.json` is enough: it holds the key column per
version, which is all you need to check a transmitted code. 17 KiB gzipped for all
27 codelists and 272 versions.

## License

MIT