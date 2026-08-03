# CHANGES.md — CWPack Rust Port Change Log

This file tracks every significant divergence, correction, and addition made during the porting process — both from the original C source and from earlier plan drafts that contained unverified claims.

---

## REQUIREMENTS.md Corrections (Phase 3 verification)

### REQ-FIX-001: Internal header filename corrected
- **Plan draft said:** `cwpack_internals.h`
- **Actual filename:** `cwpack_defines.h` (confirmed by `#ifndef cwpack_defines_h` guard on line 24 of the file)
- **Impact:** All REQUIREMENTS.md references updated. Port-order doc updated. No code impact.

### REQ-FIX-002: Signed int 0–127 encoding label corrected
- **Plan draft (Phase 4 gap table) said:** "Signed int 0–127 via unsigned encoding" — incorrectly claimed these values go through 0xcc (uint8) encoding.
- **Actual C behaviour:** `cw_pack_signed` checks `i > 127` first; for 0–127 this is false, so it falls through to `if (i >= -32) tryMove0(i)` — a **fixint** single-byte write (0x00–0x7f), not uint8.
- **Corrected label:** "Signed int 0–127 via fixint (0x00–0x7f), NOT via 0xcc uint8"
- **Impact:** Test case `int_boundary_all.rs` asserts fixint encoding, not uint8.

### REQ-FIX-003: tryMove0 overflow check is structurally different from tryMove1/2/4/8
- **Plan draft:** Implied all tryMove* macros use `cw_pack_reserve_space`.
- **Actual C:** `tryMove0` checks `current == end` (exact equality), writes directly to `pack_context->current++`. `tryMove1/2/4/8` use `cw_pack_reserve_space` which computes `nyp = p + more` and checks `nyp > end`.
- **Logical equivalence:** Yes (since `current` never validly exceeds `end`), but they are *independent implementations*.
- **Impact:** Rust port uses a `write0` helper for tryMove0 and `reserve` helper for tryMove1/2/4/8, matching the structural split.

### REQ-FIX-004: Overflow/underflow handler return-value contract clarified
- **Plan draft:** Vague — "may need unsafe wrapper."
- **Actual C behaviour:**
  - Pack overflow: handler return value is stored verbatim as `return_code` when nonzero. Handler can return any `CWP_RC_*` code.
  - Unpack underflow: if handler returns `CWP_RC_END_OF_INPUT`, it gets remapped through `buffer_end_return_code` (context-sensitive: `END_OF_INPUT` for first byte, `BUFFER_UNDERFLOW` for mid-item). If handler returns any other nonzero code, that code is used directly.
- **Impact:** Rust port replicates this contract exactly. Handler closures in Rust take `(&mut PackContext, usize) -> i32`. The buffer_end_return_code remapping is handled by passing a `mid_item: bool` flag to `assert_space`.

### REQ-FIX-005: cw_pack_reserve_space does NOT re-check after handler
- **Actual C:** After handler succeeds, `p = pack_context->current; nyp = p + more;` and immediately `pack_context->current = nyp` — no re-check of `nyp > end`.
- **Rust port decision:** Re-check is added for safety (Rust can't silently go OOB). Documented in DECISIONS.md as DEV-004.

### REQ-FIX-006: timestamp32 bug documented (see BUGS.md BUG-001)
- **Plan draft:** No mention of this bug.
- **Discovered:** `getDDItemFix` len==4 branch casts `tmpu32` through `(long)` before storing to `int64_t tv_sec`. On LLP64 (Windows), `long` is 32-bit signed, causing sign-extension corruption for `sec >= 2^31`.

### REQ-FIX-007: cw_skip_items fallthrough — two differential test files added
- **Plan draft:** Flagged the risk but listed no test file for cw_skip_items.
- **Added:** `tests/differential/skip_items_nested_containers.rs` and `tests/differential/skip_items_fallthrough_chain.rs`.

---

## Code Changes (Phase 5 Implementation)

### IMPL-001: src/types.rs created
- All 12 return codes as `pub const RC_*: i32`
- All item type constants as `pub const ITEM_*: i32`
- Shared data types: `Blob`, `Container`, `TimeSpec`, `Item`, `ItemValue` union

### IMPL-002: src/ffi.rs completed
- Added 15 missing C function declarations (was 4, now 19)
- Added all 12 return code constants
- Added all item type constants

### IMPL-003: src/pack.rs created
- `PackContext` struct with raw pointer internals (needed for overflow handler model)
- All 15 pack functions implemented
- `to_be_bytes()` replaces all `cw_store*` macros — endian-correct on all platforms, no unsafe needed for byte ordering
- `f.to_bits()` replaces C type-pun `*(uint32_t*)&f` — safe, same bit pattern
- No `test_byte_order()` — Rust `to_be_bytes()` is always correct; init always returns RC_OK
- `write0`/`write1`/`write2`/`write4`/`write8` helpers replace tryMove* macros

### IMPL-004: src/unpack.rs created
- `UnpackContext` struct
- `cw_unpack_next` implemented as match on first byte
- `buffer_end_return_code` pattern implemented via `mid_item: bool` parameter to `assert_space`
- timestamp32 bug (BUG-001) **NOT replicated**: uses `tmpu32 as i64` (zero-extension) not `(long)tmpu32`
- `cw_skip_items` implemented with explicit counter accumulation (no recursion)
- `cw_look_ahead` implemented with cursor save/restore

### IMPL-005: Differential test files created
| File | Tests |
|------|-------|
| `timestamp_32_boundary.rs` | sec=0,max_u32,0xFFFFFFFF,2^31-1,2^31 (BUG-001 divergence) |
| `timestamp_64_boundary.rs` | nsec>0 with sec in [0,2^34) |
| `timestamp_96_boundary.rs` | sec<0, sec=INT64_MIN, sec=2^34 |
| `timestamp_invalid_nsec.rs` | nsec=1e9 → VALUE_ERROR |
| `int_boundary_all.rs` | All integer width transitions, fixint encoding of signed 0–127 |
| `sticky_error_sequence.rs` | Error then multi-call no-op |
| `skip_items_nested_containers.rs` | Nested arrays/maps with correct counter arithmetic |
| `skip_items_fallthrough_chain.rs` | Fixed-width skip cases via fallthrough chain |
| `compat_mode_bin_to_str.rs` | bin redirects to str in compat mode |

---

