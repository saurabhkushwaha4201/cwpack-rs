# cwpack-rs

A safe, zero-allocation Rust port of [CWPack](https://github.com/Claes/cwpack), a fast and lightweight MessagePack C library.

`cwpack-rs` mirrors CWPack`s context-driven, buffer-pointer architecture for packing and unpacking MessagePack data. It achieves identical behavior (including sticky error handling and overflow/underflow routing) without relying on dynamically allocated memory or recursive decoding.

## Build and Test

To build the library:

```sh
cargo build --release
```

To run the differential test suite (which cross-checks the Rust implementation against the original C library via FFI):

```sh
cargo test
```

## Known Bugs and Deviations

The Rust port is strictly 100% behaviorally equivalent to the C implementation, with one intentional exception:
- **BUG-001**: A sign-extension bug exists in the original C library when unpacking 32-bit timestamps on LLP64 platforms (e.g. Windows). For timestamps where `sec >= 2^31`, the C code erroneously returns a negative number. `cwpack-rs` corrects this to correctly return the unsigned positive value.

For full technical details on struct layout mappings, toolchain handling, and specific C macro translations (such as the `cw_skip_bytes` fallthrough implementation), please see [`DECISIONS.md`](DECISIONS.md) and [`BUGS.md`](BUGS.md).

## Fuzzing

Fuzzing targets the round-trip encoding and decoding (pack -> unpack) and validates structural equality between the Rust implementation and the C Oracle.

To run the fuzzer locally (requires nightly Rust and a compatible LLVM/Clang toolchain):

```sh
rustup default nightly
cargo fuzz run roundtrip_vs_ffi
```

*(This fuzz target is also executed automatically via GitHub Actions CI on `ubuntu-latest`).*
