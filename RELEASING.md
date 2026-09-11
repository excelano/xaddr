# Releasing xaddr

The release loop lives in `~/notes/releasing.md`; failure recipes are in
`~/notes/build_release_gotchas.md`. This file carries what is true of xaddr and
not of its siblings, which for a library is most of it — there are no platform
tarballs, no installers, no `.deb`, no Homebrew formula and no winget manifest.
crates.io is the only channel and docs.rs builds itself from it.

| | |
|---|---|
| Loop | crate-only |
| Version lives in | `Cargo.toml` |
| crates | `xaddr` |

**Check what a consumer will see before tagging.** `cargo publish --dry-run`
packages the crate and compiles it from the packaged copy, which catches a file
`.gitignore` excluded and the build needed. `cargo doc --no-deps --open` is
worth the look too: the crate's documentation is its interface.

**There is no GitHub release, so the tag is the whole of step 2.** The `v*` tag
triggers `publish-crate.yml` and crates.io is live within a minute. Nothing
creates a release page here and nothing should — a library with no artefacts
would be a page with nothing on it.

**Bump the consumers in the same sitting.** This is the step a library release
loses, and it loses it silently: xled, xshape and Comma keep building against
the old version and simply do not have the fix. Nothing fails, and `fleet -r`
cannot see it — it compares tags to releases and this is neither. Update the
`xaddr` requirement in each, run its tests, and release those on their own
loops.

## When a change to the grammar is a breaking change

The grammar is the interface, more than the Rust signatures are. Widening it —
a new address form, an error that becomes an acceptance — is a minor bump, and
consumers pick it up whenever they upgrade. Narrowing it is a breaking change
even when the types are untouched, because an address a user has in a script or
a saved query stops working. `tests/conformance.rs` is the record of what has
been promised; a case removed or changed there is the signal to think about the
version number, not a test to update in passing.
