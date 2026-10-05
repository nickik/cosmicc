# Reproducible local SIA portability fix

`sia32-frame-and-branches.patch` applies to published crainlift revision
`9edf4ede3728b51c2abf21ce4686bd4ed0a391ae`. It preserves reserved r15 before
replacing the caller frame pointer, excludes r15 from allocation, and terminates
distant backward branches using a full-width PC-relative veneer. It includes
regressions for frame-save order and long-branch patching.

Apply with `git am` in an isolated backend checkout at that exact base. Local
validation used `/tmp/cosmic-backend-frame-fix`; it leaves the sibling working
checkout untouched. To build Cosmic C against that checkout, use these explicit
Cargo overrides (adjust the absolute checkout path as needed):

```sh
cargo build --offline --bin cosmicc \
  --config 'patch."https://github.com/nickik/crainlift".cranelift-codegen.path="/tmp/cosmic-backend-frame-fix/cranelift/codegen"' \
  --config 'patch."https://github.com/nickik/crainlift".cranelift-frontend.path="/tmp/cosmic-backend-frame-fix/cranelift/frontend"' \
  --config 'patch."https://github.com/nickik/crainlift".target-lexicon.path="/tmp/cosmic-backend-frame-fix/vendor/target-lexicon-sia"'
```

The patch's local commit is `5df9a089753b520a47251c635e99be4466b4e7f6`.
40 SIA tests pass. Expanded unmodified upstream zlib CRC passes architectural
and composed-board execution with stack/frame restoration. See
../../REAL_SOFTWARE_STATUS.md for exact evidence and remaining limitations.

The user authorized publication and the patch is now published on
`https://github.com/nickik/crainlift/tree/cosmicc-frame-preservation`.
Cosmic C's manifests and lockfile pin the exact revision above. Local-override
instructions remain useful for reproducing the isolated patch itself.
