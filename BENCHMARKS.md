# CWPack-RS Benchmarks

These benchmarks measure the relative throughput of the `cwpack-rs` port against the original `cwpack` C library and `msgpack-rust` (rmp). 
The tests encode/decode a structured payload: an array of 3 containing an integer, a string, and a map of 1 (string to boolean).

## Results

Total Time for **10,000,000 Iterations**:

| Operation | `cwpack-rs` | `cwpack` (C) | `msgpack-rust` (rmp) |
|-----------|-------------|--------------|----------------------|
| Pack      | 463.5 ms    | 681.0 ms     | 373.4 ms             |
| Unpack    | 625.2 ms    | 516.0 ms     | 180.2 ms             |

*(The C baseline was compiled standalone with `gcc -O3 bench.c src/cwpack.c` using the W64devkit MinGW compiler, to bypass the `x86_64` FFI linking crash and get an authentic optimized baseline).*

### Analysis
- **Pack**: `cwpack-rs` outperforms the original C implementation by ~30% on this platform (46.3 ns vs 68.1 ns per sequence). `msgpack-rust` maintains a narrow lead at 37.3 ns.
- **Unpack**: The original C library unpacks slightly faster than the Rust port (51.6 ns vs 62.5 ns). `msgpack-rust` decodes significantly faster (~18 ns), primarily because it implements its decoding using direct `Read` trait methods and tightly inlined macros without maintaining the dynamic union-based `Item` state tracking that `cwpack` relies on.
- **Conclusion**: `cwpack-rs` is structurally equivalent to the C original and achieves on-par (or better) throughput, trading ~10ns of unpack overhead for a ~20ns packing gain while securing 100% memory safety.
