# Releasing

The libraries are released with [release-plz](https://release-plz.dev/), each on its own:
its own version, its own `CHANGELOG.md` (in its crate's directory), its own tag
`<crate>-v<version>` and GitHub release (`release-plz.toml`). An app takes the widgets it
uses, and a break in one must not bump another. The demo app and, until its editor is
written, alumina-markdown are not released (`publish = false`).

## Normal flow

1. On every push to `main`, release-plz opens or updates a release PR (labelled `release`):
   the next version of each crate that changed, the updated `Cargo.lock`, and a section for
   the version in each one's `CHANGELOG.md`, from the conventional commits since its last
   release.
2. Review the release PR: the versions, the changelogs.
3. Merge it: release-plz publishes the crates to crates.io, tags them and makes their GitHub
   releases, each one's notes its `CHANGELOG.md` section.

Pull requests do not touch the changelogs: their commit messages are what they are written
from, so write them for a reader of the changelog, scoped by crate
(`feat(alumina-datetime): …`).

### The first release

0.1.0 is the version alumina-datetime already carries, so the first release PR keeps it.
Tidy its changelog section there, once.

### Releasing alumina-markdown

Remove `publish = false` from its `Cargo.toml` in the pull request that makes the editor
usable; release-plz takes it up from the next push to `main`.

## Version bump rules

release-plz infers the version from conventional commits (pre-1.0, `0.x.y`):

- `feat:`, `fix:` and other non-breaking changes → patch bump.
- `feat!:` / `fix!:`, or a `BREAKING CHANGE:` footer → minor bump.
- A change to the public API (a signature, a public field, a removed item) is breaking:
  mark its commit with `!`.

Apps must use the same egui major as these crates, so a bump of egui is a breaking change
too: make it a minor bump, and add the row to the crate's README's version table.

## Checking that the crates publish

```sh
cargo publish --workspace --dry-run
```

packages and builds each publishable crate on its own, as crates.io will (CI runs it too).
`release-plz update` makes the release PR's changes in the working tree, to see them before
a push.

## Secrets

The workflow uses the sapphire-turtle organization's `RELEASE_PLZ_TOKEN` (a fine-grained PAT
with Contents and Pull requests read/write on this repository; the release PR is opened
with it, not `GITHUB_TOKEN`, so CI runs on it) and `CARGO_REGISTRY_TOKEN`.
