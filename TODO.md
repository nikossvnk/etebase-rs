# TODO

This file tracks planned changes to the project, especially breaking changes that have to wait
until the next major release.

## Planned changes for next major release

- Change argument types in `src/service.rs` for public and private keys to use `[u8; 32]` instead
  of `[u8]`. This should remove all the `ProgrammingError`s resulting from failed `try_into()` calls.

## Dependency maintenance

- Replace `sodiumoxide`, which is deprecated and unmaintained (RUSTSEC-2021-0137). The output must
  stay byte-compatible with data already encrypted on Etebase servers, so first add golden-vector
  tests recorded with the current build, covering argon2id key derivation,
  `crypto_aead_xchacha20poly1305_ietf`, `crypto_box`, `crypto_sign`, keyed `crypto_generichash`
  (BLAKE2b) and padding. Then port to `libsodium-sys-stable` (same libsodium, least risk), or to the
  RustCrypto crates, and verify against a test server using data created by the old build.
