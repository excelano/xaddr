# Releasing xaddr

The release loop for a new version. Run it from a clean `main` with the working tree committed. Examples below cut `v0.2.0`; substitute the version you are actually releasing.

xaddr is a library, so this is much shorter than the CLI release loops. There are no platform tarballs, no installers, no `.deb`, no Homebrew formula, and no winget manifest — crates.io is the only channel, and `docs.rs` builds itself from it.

1. **Bump the version.** Edit `version` in `Cargo.toml`. Update `Cargo.lock` with a build (`cargo build`), run `cargo test`, commit.

2. **Check what consumers will see.** `cargo publish --dry-run` packages the crate and compiles it from the packaged copy, which catches a file the `.gitignore` excluded but the build needed. `cargo doc --no-deps --open` is worth a look too, since the crate's documentation is its interface.

3. **Tag and push.** `git tag v0.2.0 && git push origin main --tags`. The `v*` tag triggers `publish-crate.yml`, which runs `cargo publish` with the org-secret token, so crates.io is live within a minute of the push and there is no local publish step. Confirm it succeeded:
   ```sh
   gh run list --workflow=publish-crate.yml --limit 1
   ```
   Do **not** run `cargo publish` by hand — the pipeline beats you to it and you will just get `already exists`. **Versions are immutable**: you can `cargo yank` a bad release to hide it from new dependency resolution, but never re-publish the same number. A fix is always a fresh version bump.

4. **Bump the consumers.** This is the step a library release loses, because nothing fails when you skip it — xled, xshape, and Comma keep building against the old version and simply do not have the fix. Update the `xaddr` requirement in each repo that depends on it, run its tests, and release those on their own loops. `fleet -r` will not catch this one, so it belongs in the same sitting as the tag.

## When a change to the grammar is a breaking change

The grammar is the interface, more than the Rust signatures are. Widening it — a new address form, an error that becomes an acceptance — is a minor bump, and consumers pick it up whenever they upgrade. Narrowing it is a breaking change even when the types are untouched, because an address a user has in a script or a saved query stops working. `tests/conformance.rs` is the record of what has been promised; a case removed or changed there is the signal to think about the version number, not a test to update in passing.
