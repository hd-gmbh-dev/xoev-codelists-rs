# Trusted publishers — setup instructions (todo #6, step B)

Everything below is one-time UI work; the workflows are already prepared
(`publish.yaml` prefers OIDC and only falls back to the tokens until you remove them).

Common values for every entry:

| field | value |
|---|---|
| GitHub organization | `hd-gmbh-dev` |
| Repository | `xoev-codelists-rs` |
| Workflow filename | `publish.yaml` |
| Environment | *(leave empty — the workflow does not declare one)* |

---

## 1. crates.io — 7 crates

Prerequisite: you must be an **owner** of the crate (they were published with the
`CRATES_TOKEN`), and trusted publishing can only be configured *after* the first
publish — both are true for the `0.1.0-alpha.1` versions already on crates.io.

For each crate:

1. Sign in at <https://crates.io> (GitHub account `kilork`).
2. Open the crate page:
   - <https://crates.io/crates/xoev-codelists>
   - <https://crates.io/crates/xoev-codelists-xwasser>
   - <https://crates.io/crates/xoev-codelists-destatis>
   - <https://crates.io/crates/xoev-codelists-xunternehmen>
   - <https://crates.io/crates/xoev-codelists-dsmeld>
   - <https://crates.io/crates/xoev-codelists-xauslaender>
   - <https://crates.io/crates/xoev-codelists-bmi-bit>
3. Open the **Settings** tab of that crate.
4. In the **Trusted Publishing** section click **Add**.
5. Fill in the three fields from the table above (repository owner, repository
   name, workflow filename), leave the environment empty, and confirm.
6. The entry then shows up in that crate's trusted publisher list — repeat for the
   remaining six.

Sanity check afterwards: each of the seven crate settings pages lists
`hd-gmbh-dev/xoev-codelists-rs` with workflow `publish.yaml`.

## 2. npm — 1 package

1. Sign in at <https://www.npmjs.com> (account that owns `xoev-codelists`, 2FA
   enabled — npm requires it for settings changes).
2. Open <https://www.npmjs.com/package/xoev-codelists> → **Settings**.
3. Find **Trusted Publisher** (GitHub Actions) and add:
   - Organization or user: `hd-gmbh-dev`
   - Repository: `xoev-codelists-rs`
   - Workflow filename: `publish.yaml`
   - Environment name: *(empty)*
4. **Allowed actions:** enable **`npm publish`** (a new trusted publisher can only
   stage by default). We want direct publishing — the package is generated and
   verified in CI, and a per-release manual approval would break the unattended
   refresh.

CLI alternative (npm ≥ 11.15, after `npm login`):

```sh
npm trust github xoev-codelists \
  --repo hd-gmbh-dev/xoev-codelists-rs \
  --file publish.yaml \
  --allow-publish
```

## 3. What happens next (by me)

1. Remove the `CRATES_TOKEN` / `NPM_TOKEN` fallbacks from `publish.yaml` so a stale
   secret cannot mask a broken OIDC setup.
2. `gh workflow run publish.yaml` is *not* a useful test (everything is already
   published → all steps skip). The real test is the release itself: push tag
   `v1.2610.0`, which publishes all 7 crates + npm and then runs the new
   **Verify published artifacts** job (installs the published crates and npm
   package, asserts `contains(urn, "1", "1760") == Some(true)` etc.).
3. Only when that is green: delete the secrets and note the migration done in #1.

## Verification checklist

- [ ] 7 crate settings pages list the trusted publisher
- [ ] npm package settings list the trusted publisher with `npm publish` allowed
- [ ] `publish.yaml` contains no `--token` / `NODE_AUTH_TOKEN` (after my change)
- [ ] tag `v1.2610.0` publishes and the verify job passes
- [ ] `CRATES_TOKEN` / `NPM_TOKEN` deleted from the repo secrets
