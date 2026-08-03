# CWPack Original — Bug Register

## BUG-001: timestamp32 unpack sign-extends through platform-width `long` on LLP64

**Found via:** Manual review of `getDDItemFix` macro in `original/src/cwpack_defines.h` (line 349), confirmed by ABI analysis and cross-reference with Windows LLP64 / Linux LP64 ABI documentation.

**Location:** `original/src/cwpack_defines.h`, `getDDItemFix` macro, `len==4` (timestamp32) branch

**Exact line:**
```c
unpack_context->item.as.time.tv_sec = (long)tmpu32;   // line 349
```

**Root cause:** `tmpu32` is `uint32_t`. `tv_sec` is `int64_t`. The cast goes through `long` — whose width is **platform-dependent**:
- **Windows LLP64** (including this project's w64devkit toolchain): `long` = 32 bits, signed
- **Linux/macOS LP64**: `long` = 64 bits, signed

**Repro:** Unpack a timestamp32 message with `sec >= 2^31` (e.g. `sec = 3,000,000,000`) on a platform where `long` is 32-bit.

**Step-by-step on Windows LLP64:**
1. `tmpu32 = 3000000000` (0xB2D05E00) — valid `uint32_t` value
2. `(long)tmpu32` → converts to 32-bit signed long → -1294967296 (overflows, sign-extended)
3. Stored into 64-bit `tv_sec` as -1294967296 — **wrong negative value**

**Correct behavior (per MessagePack spec):** `tv_sec` in timestamp32 is an unsigned 32-bit seconds count, always 0–4294967295. Any `sec` in range [0, 2³²) must unpack as non-negative. The value should zero-extend, not sign-extend.

**Affected range:** Any timestamp32 value where `sec ∈ [2,147,483,648, 4,294,967,295]` — roughly half of timestamp32's valid range.

**Platform scope:** Bug is **silent** on LP64 (Linux/macOS) because 64-bit `long` zero-extends correctly. Bug is **active** on LLP64 (Windows, WASM) and any 32-bit platform.

**Status:** Confirmed bug in original C. **NOT reproduced in Rust port** — Rust has no platform-varying integer type. The port casts `tmpu32` as `u32` zero-extended directly to `i64`, which is correct on every platform by construction.

**Differential test behaviour:** `tests/differential/timestamp_32_boundary.rs` will **intentionally diverge** from the Windows-compiled C oracle for `sec >= 2^31`. This divergence is the bug being caught, not a defect in the Rust port. Test comments mark this explicitly.

---

## BUG-002: `cw_pack_insert` bypasses sticky-error guard

**Found via:** Code review of `cwpack.c` line 429–434.

**Location:** `original/src/cwpack.c`, `cw_pack_insert` function

**Exact code:**
```c
void cw_pack_insert (cw_pack_context* pack_context, const void* v, uint32_t l)
{
    uint8_t *p;
    cw_pack_reserve_space(l);     // ← no return_code check before this
    memcpy(p,v,l);
}
```

**Root cause:** Unlike every other pack function, `cw_pack_insert` does NOT check `return_code` before writing. It will write raw bytes into the buffer even when the context is in an error state. This can silently corrupt partially-written data after a prior error.

**Correct behavior:** N/A — this is intentional design, not a defect.

**Severity:** Low in normal usage (callers checking `return_code` after each call). Medium if used in a pipeline that assumes post-error calls are no-ops.

**Status:** Intentional design (escape hatch for pre-encoded MsgPack blobs). The Rust port **replicates** this behaviour faithfully, with an explicit comment. Documented in DECISIONS.md. Verified by tests/differential/cw_pack_insert_bypasses.rs.

---

## BUG-003: Integer overflow in `cw_pack_reserve_space` for large strings/blobs

**Found via:** Code review of string/bin/ext packing macros and `cw_pack_reserve_space`.

**Location:** `original/src/cwpack_defines.h` (`cw_pack_reserve_space`) and three locations in `original/src/cwpack.c`:
- `cw_pack_str` (line 276): `cw_pack_reserve_space(l+5)`
- `cw_pack_bin` (line 314): `cw_pack_reserve_space(l+5)`
- `cw_pack_ext` (line 372): `cw_pack_reserve_space(l+6)`

**Exact code:**
```c
    // In cw_pack_str (line 276):
    cw_pack_reserve_space(l+5)
    *p++ = (uint8_t)0xdb;
    cw_store32(l);
    memcpy(p+4,v,l);
```

**Root cause:** The length `l` is a `uint32_t`. When `l` is extremely large (e.g., `0xFFFFFFFF`), the expression `l+5` overflows 32-bit arithmetic and wraps around to a small number (e.g., `4`). 
The macro `cw_pack_reserve_space(4)` checks if there are 4 bytes available. If true, it bypasses the overflow handler, advances the current pointer by 4, and then `memcpy` attempts to write `0xFFFFFFFF` bytes into the buffer, resulting in a massive heap/stack buffer overflow.

**Severity:** High (Buffer Overflow). If an attacker can control the length parameter of a string/bin/ext payload (even with a dummy pointer if just fuzzing the packer), they can bypass bounds checking entirely.

**Status:** Confirmed bug in original C. **NOT reproduced in Rust port**. In `cwpack-rs`, `data` is passed as a safe slice `&[u8]`. On 32-bit targets, Rust slices are strictly limited to `isize::MAX` (`0x7FFFFFFF`), so `l as usize + 5` maxes out at `0x80000004`, which fits perfectly within a 32-bit `usize` without wrapping. The overflow check will correctly trigger the handler.

---
