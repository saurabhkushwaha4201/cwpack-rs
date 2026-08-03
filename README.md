# 📦 cwpack-rs

![Build Status](https://github.com/saurabhkushwaha4201/cwpack-rs/actions/workflows/ci.yml/badge.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

A safe, **zero-allocation** Rust port of [CWPack](https://github.com/Claes/cwpack), a blazing fast and lightweight MessagePack C library.

`cwpack-rs` mirrors CWPack's context-driven, buffer-pointer architecture for packing and unpacking MessagePack data. It achieves identical behavior (including sticky error handling and overflow/underflow routing) without relying on dynamically allocated memory or recursive decoding.

---

<details open>
<summary><b>📑 Table of Contents</b></summary>

- [Features](#-features)
- [Quick Start](#-quick-start)
- [Benchmarks](#-benchmarks)
- [Build and Test](#-build-and-test)
- [Fuzzing](#-fuzzing)
- [Known Bugs & Deviations](#-known-bugs-and-deviations)
</details>

---

## ✨ Features

- **Zero Allocation**: Operates entirely on borrowed `&[u8]` and `&mut [u8]` buffers via its public API. No `Vec` or `String` allocations inside the core packer/unpacker.
- **Memory Safety**: Structurally prevents C's integer overflows (like BUG-003) by relying on Rust's strict slice length guarantees at the API boundary.
- **Context-Driven**: Stateful packing and unpacking with "sticky" error contexts, meaning you can chain operations and check for errors once at the end.
- **C-Oracle Verified**: Tested directly against the original C implementation via differential FFI fuzzing.

## 🚀 Quick Start

### Packing Data
```rust
use cwpack_rs::pack::PackContext;

let mut buffer = [0u8; 1024];
let mut ctx = PackContext::new(&mut buffer);

// Pack an array of 3 items
ctx.pack_array(3);
ctx.pack_int(42);
ctx.pack_str("Hello");
ctx.pack_boolean(true);

// Check if any errors occurred during the sequence
if ctx.return_code == cwpack_rs::types::RC_OK {
    let packed_data = ctx.get_valid_data();
    println!("Successfully packed {} bytes!", packed_data.len());
}
```

### Unpacking Data
```rust
use cwpack_rs::unpack::UnpackContext;

// Assume `packed_data` is a &[u8] slice containing MessagePack
let mut ctx = UnpackContext::new(packed_data);

// Unpack items one by one
ctx.unpack_next();
if ctx.return_code == cwpack_rs::types::RC_OK {
    println!("First item: {:?}", ctx.item);
}
```

## ⚡ Benchmarks

`cwpack-rs` performs on-par or better than the original C library!

| Operation | `cwpack-rs` | `cwpack` (C) |
|-----------|-------------|--------------|
| Pack      | **46.3 ns** | 68.1 ns      |
| Unpack    | 62.5 ns     | **51.6 ns**  |

> 📈 *For deep-dive performance analysis and comparisons with `msgpack-rust`, see the full [BENCHMARKS.md](BENCHMARKS.md).*

## 🛠️ Build and Test

To build the library in release mode:
```sh
cargo build --release
```

To run the differential test suite, which rigorously cross-checks the Rust implementation against the original C library via FFI bindings:
```sh
cargo test
```

## 👾 Fuzzing

Fuzzing targets the round-trip encoding and decoding (pack -> unpack) and validates structural equality between the Rust implementation and the C Oracle.

<details>
<summary><b>Instructions to run fuzzing locally</b></summary>

Requires nightly Rust and a compatible LLVM/Clang toolchain:
```sh
rustup default nightly
cargo fuzz run roundtrip_vs_ffi -- -max_total_time=60
```
*(This fuzz target is also executed automatically via GitHub Actions CI on `ubuntu-latest`).*
</details>

## 🐛 Known Bugs and Deviations

The Rust port is strictly 100% behaviorally equivalent to the C implementation, **except where it fixes critical bugs found in the C Oracle**.

<details open>
<summary><b>Fixed C-Oracle Bugs</b></summary>

- **BUG-001**: A sign-extension bug in the original C library when unpacking 32-bit timestamps on LLP64 platforms (e.g. Windows). 
- **BUG-003**: A catastrophic integer overflow in `cw_pack_reserve_space` leading to buffer overflows for large payloads.
</details>

> 🔍 *For detailed write-ups on these bugs, read [BUGS.md](BUGS.md).*
> 📐 *For full technical details on struct layout mappings, toolchain handling, and specific C macro translations, please see [DECISIONS.md](DECISIONS.md).*

---
