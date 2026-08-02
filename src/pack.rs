//! CWPack packer — Rust port of cwpack.c pack-side functions.
//!
//! Design decisions (see DECISIONS.md):
//! - Raw pointer internals (`current`, `start`, `end`) to support the
//!   overflow-handler model where a handler can replace the buffer pointer.
//! - `to_be_bytes()` replaces all `cw_store*` macros (DEV-006).
//! - `f.to_bits()` replaces C type-pun (DEV-007).
//! - No `test_byte_order()` — Rust is always endian-correct (DEV-006).
//! - `cw_pack_insert` has NO sticky-error guard (DEV-002 / BUG-002).
//! - After overflow handler, buffer bounds are re-checked for safety (DEV-004).

use crate::types::{RC_BUFFER_OVERFLOW, RC_ILLEGAL_CALL, RC_OK, RC_VALUE_ERROR};
use std::ptr;

/// Overflow handler: given (context, bytes_needed), must extend the buffer and
/// return RC_OK, or return any CWP_RC_* error code. Return value stored verbatim
/// as return_code (DEV-003). The handler may update `current`, `start`, `end`.
pub type PackOverflowHandler = unsafe fn(*mut PackContext, usize) -> i32;
/// Flush handler: called by pack_flush. Return value stored verbatim as return_code.
pub type PackFlushHandler = unsafe fn(*mut PackContext) -> i32;

/// Mirrors `cw_pack_context` exactly (field names match C original).
/// Raw pointer internals needed to support overflow handlers that repoint
/// the buffer (the same model used in C).
pub struct PackContext {
    pub current: *mut u8,
    pub start: *mut u8,
    pub end: *mut u8,
    pub be_compatible: bool,
    pub return_code: i32,
    pub err_no: i32,
    pub overflow_handler: Option<PackOverflowHandler>,
    pub flush_handler: Option<PackFlushHandler>,
}

// SAFETY: PackContext does not use thread-local state. Caller is responsible
// for not sharing across threads without synchronization.
unsafe impl Send for PackContext {}

impl PackContext {
    /// Initialise from a mutable byte slice. No overflow handler.
    /// Mirrors `cw_pack_context_init` with `hpo = NULL`.
    pub fn new(buf: &mut [u8]) -> Self {
        let start = buf.as_mut_ptr();
        let end = unsafe { start.add(buf.len()) };
        PackContext {
            start,
            current: start,
            end,
            be_compatible: false,
            return_code: RC_OK, // DEV-006: no test_byte_order needed
            err_no: 0,
            overflow_handler: None,
            flush_handler: None,
        }
    }

    /// Initialise with an overflow handler (advanced use).
    pub fn with_overflow_handler(
        buf: &mut [u8],
        handler: PackOverflowHandler,
    ) -> Self {
        let mut ctx = Self::new(buf);
        ctx.overflow_handler = Some(handler);
        ctx
    }

    /// Set compatibility mode (affects str, bin, ext, time).
    /// No sticky-error guard (can be set at any time).
    pub fn set_compatibility(&mut self, be_compatible: bool) {
        self.be_compatible = be_compatible;
    }

    /// Set a flush handler.
    /// No sticky-error guard.
    pub fn set_flush_handler(&mut self, handler: PackFlushHandler) {
        self.flush_handler = Some(handler);
    }

    /// Number of bytes written so far.
    #[inline]
    pub fn written(&self) -> usize {
        unsafe { self.current.offset_from(self.start) as usize }
    }

    /// View the written bytes (safe because we own the buffer via init).
    /// # Safety
    /// The returned slice is valid for as long as the original buffer is live.
    pub unsafe fn written_bytes(&self) -> &[u8] {
        std::slice::from_raw_parts(self.start, self.written())
    }

    // -----------------------------------------------------------------------
    // Private helpers — inline equivalents of C macro family
    // -----------------------------------------------------------------------

    /// tryMove0: write one byte (format byte only). Checks current == end.
    /// Structurally different from tryMove1/2/4/8 (see REQ-FIX-003, CHANGES.md).
    #[inline(always)]
    unsafe fn write0(&mut self, tag: u8) {
        if self.current == self.end {
            match self.overflow_handler {
                None => {
                    self.return_code = RC_BUFFER_OVERFLOW;
                    return;
                }
                Some(handler) => {
                    let rc = handler(self as *mut PackContext, 1);
                    if rc != RC_OK {
                        self.return_code = rc;
                        return;
                    }
                    // After successful handler, re-check (DEV-004)
                    if self.current == self.end {
                        self.return_code = RC_BUFFER_OVERFLOW;
                        return;
                    }
                }
            }
        }
        *self.current = tag;
        self.current = self.current.add(1);
    }

    /// tryMove1/2/4/8: reserve `more` bytes, return pointer to them or set error.
    /// Uses `cw_pack_reserve_space` semantics with handler support (DEV-004).
    #[inline(always)]
    unsafe fn reserve(&mut self, more: usize) -> Option<*mut u8> {
        let p = self.current;
        let nyp = p.add(more);
        if nyp > self.end {
            match self.overflow_handler {
                None => {
                    self.return_code = RC_BUFFER_OVERFLOW;
                    return None;
                }
                Some(handler) => {
                    let rc = handler(self as *mut PackContext, more);
                    if rc != RC_OK {
                        self.return_code = rc;
                        return None;
                    }
                    // Re-check after handler (DEV-004; C original skips this)
                    let p2 = self.current;
                    let nyp2 = p2.add(more);
                    if nyp2 > self.end {
                        self.return_code = RC_BUFFER_OVERFLOW;
                        return None;
                    }
                    self.current = nyp2;
                    return Some(p2);
                }
            }
        }
        self.current = nyp;
        Some(p)
    }

    /// tryMove1: tag + 1 data byte.
    #[inline(always)]
    unsafe fn write1(&mut self, tag: u8, data: u8) {
        if let Some(p) = self.reserve(2) {
            *p = tag;
            *p.add(1) = data;
        }
    }

    /// tryMove2: tag + 2 data bytes (big-endian).
    #[inline(always)]
    unsafe fn write2(&mut self, tag: u8, data: u16) {
        if let Some(p) = self.reserve(3) {
            *p = tag;
            let b = data.to_be_bytes();
            *p.add(1) = b[0];
            *p.add(2) = b[1];
        }
    }

    /// tryMove4: tag + 4 data bytes (big-endian).
    #[inline(always)]
    unsafe fn write4(&mut self, tag: u8, data: u32) {
        if let Some(p) = self.reserve(5) {
            *p = tag;
            ptr::copy_nonoverlapping(data.to_be_bytes().as_ptr(), p.add(1), 4);
        }
    }

    /// tryMove8: tag + 8 data bytes (big-endian).
    #[inline(always)]
    unsafe fn write8(&mut self, tag: u8, data: u64) {
        if let Some(p) = self.reserve(9) {
            *p = tag;
            ptr::copy_nonoverlapping(data.to_be_bytes().as_ptr(), p.add(1), 8);
        }
    }

    // -----------------------------------------------------------------------
    // Public pack functions (all with sticky-error no-op guard)
    // -----------------------------------------------------------------------

    /// Emit nil (`0xc0`).
    pub fn pack_nil(&mut self) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe { self.write0(0xc0) }
    }

    /// Emit true (`0xc3`).
    pub fn pack_true(&mut self) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe { self.write0(0xc3) }
    }

    /// Emit false (`0xc2`).
    pub fn pack_false(&mut self) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe { self.write0(0xc2) }
    }

    /// Emit boolean.
    pub fn pack_boolean(&mut self, b: bool) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe { self.write0(if b { 0xc3 } else { 0xc2 }) }
    }

    /// Encode `u64` using smallest representation.
    /// fixuint (≤127) / uint8 / uint16 / uint32 / uint64.
    pub fn pack_unsigned(&mut self, i: u64) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe {
            if i < 128 {
                self.write0(i as u8);
                return;
            }
            if i < 256 {
                self.write1(0xcc, i as u8);
                return;
            }
            if i < 0x10000 {
                self.write2(0xcd, i as u16);
                return;
            }
            if i < 0x1_0000_0000 {
                self.write4(0xce, i as u32);
                return;
            }
            self.write8(0xcf, i);
        }
    }

    /// Encode `i64` using smallest representation.
    /// For i > 127: uses unsigned encoding (no negative fixint for positives).
    /// For -32 ≤ i ≤ 127: fixint (single byte, wraps to 0xe0–0xff for negatives).
    /// NOTE: signed 0–127 → fixint (NOT uint8). See REQ-FIX-002 / CHANGES.md.
    pub fn pack_signed(&mut self, i: i64) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe {
            if i > 127 {
                // Positive large values — use unsigned encoding
                if i < 256 {
                    self.write1(0xcc, i as u8);
                    return;
                }
                if i < 0x10000 {
                    self.write2(0xcd, i as u16);
                    return;
                }
                if i < 0x1_0000_0000 {
                    self.write4(0xce, i as u32);
                    return;
                }
                self.write8(0xcf, i as u64);
                return;
            }
            // -32 ≤ i ≤ 127: fixint (covers 0..127 and -32..-1)
            if i >= -32 {
                self.write0(i as u8);
                return;
            }
            if i >= -128 {
                self.write1(0xd0, i as u8);
                return;
            }
            if i >= -32768 {
                self.write2(0xd1, i as u16);
                return;
            }
            // i32::MIN as i64 = -2147483648 = 0xffffffff80000000 (matches C)
            if i >= i32::MIN as i64 {
                self.write4(0xd2, i as u32);
                return;
            }
            self.write8(0xd3, i as u64);
        }
    }

    /// Encode f32. Always 5 bytes (0xca + 4). Uses f.to_bits() (DEV-007).
    pub fn pack_float(&mut self, f: f32) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe { self.write4(0xca, f.to_bits()) }
    }

    /// Encode f64. Always 9 bytes (0xcb + 8). Uses d.to_bits() (DEV-007).
    pub fn pack_double(&mut self, d: f64) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe { self.write8(0xcb, d.to_bits()) }
    }

    /// Encode array size header. fixarray (n<16) / array16 / array32.
    pub fn pack_array_size(&mut self, n: u32) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe {
            if n < 16 {
                self.write0(0x90 | n as u8);
                return;
            }
            if n < 65536 {
                self.write2(0xdc, n as u16);
                return;
            }
            self.write4(0xdd, n);
        }
    }

    /// Encode map size header. fixmap (n<16) / map16 / map32.
    pub fn pack_map_size(&mut self, n: u32) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe {
            if n < 16 {
                self.write0(0x80 | n as u8);
                return;
            }
            if n < 65536 {
                self.write2(0xde, n as u16);
                return;
            }
            self.write4(0xdf, n);
        }
    }

    /// Encode a UTF-8 string (no UTF-8 validation — matches C design).
    /// fixstr (l<32) / str8 (l<256, non-compat) / str16 / str32.
    /// In compat mode: str8 is skipped; jumps to str16 for l in [32, 65535].
    pub fn pack_str(&mut self, data: &[u8]) {
        if self.return_code != RC_OK {
            return;
        }
        let l = data.len() as u32;
        unsafe {
            if l < 32 {
                if let Some(p) = self.reserve(l as usize + 1) {
                    *p = 0xa0 | l as u8;
                    ptr::copy_nonoverlapping(data.as_ptr(), p.add(1), l as usize);
                }
                return;
            }
            if l < 256 && !self.be_compatible {
                if let Some(p) = self.reserve(l as usize + 2) {
                    *p = 0xd9;
                    *p.add(1) = l as u8;
                    ptr::copy_nonoverlapping(data.as_ptr(), p.add(2), l as usize);
                }
                return;
            }
            if l < 65536 {
                if let Some(p) = self.reserve(l as usize + 3) {
                    *p = 0xda;
                    let b = (l as u16).to_be_bytes();
                    *p.add(1) = b[0];
                    *p.add(2) = b[1];
                    ptr::copy_nonoverlapping(data.as_ptr(), p.add(3), l as usize);
                }
                return;
            }
            if let Some(p) = self.reserve(l as usize + 5) {
                *p = 0xdb;
                ptr::copy_nonoverlapping(l.to_be_bytes().as_ptr(), p.add(1), 4);
                ptr::copy_nonoverlapping(data.as_ptr(), p.add(5), l as usize);
            }
        }
    }

    /// Encode binary data. In compat mode → redirects to pack_str.
    /// bin8 / bin16 / bin32 (non-compat only).
    pub fn pack_bin(&mut self, data: &[u8]) {
        if self.return_code != RC_OK {
            return;
        }
        if self.be_compatible {
            // Compatibility mode: bin becomes str
            self.pack_str(data);
            return;
        }
        let l = data.len() as u32;
        unsafe {
            if l < 256 {
                if let Some(p) = self.reserve(l as usize + 2) {
                    *p = 0xc4;
                    *p.add(1) = l as u8;
                    ptr::copy_nonoverlapping(data.as_ptr(), p.add(2), l as usize);
                }
                return;
            }
            if l < 65536 {
                if let Some(p) = self.reserve(l as usize + 3) {
                    *p = 0xc5;
                    let b = (l as u16).to_be_bytes();
                    *p.add(1) = b[0];
                    *p.add(2) = b[1];
                    ptr::copy_nonoverlapping(data.as_ptr(), p.add(3), l as usize);
                }
                return;
            }
            if let Some(p) = self.reserve(l as usize + 5) {
                *p = 0xc6;
                ptr::copy_nonoverlapping(l.to_be_bytes().as_ptr(), p.add(1), 4);
                ptr::copy_nonoverlapping(data.as_ptr(), p.add(5), l as usize);
            }
        }
    }

    /// Encode an ext type. In compat mode → ILLEGAL_CALL.
    /// fixext1/2/4/8/16 / ext8 / ext16 / ext32.
    pub fn pack_ext(&mut self, ext_type: i8, data: &[u8]) {
        if self.return_code != RC_OK {
            return;
        }
        if self.be_compatible {
            self.return_code = RC_ILLEGAL_CALL;
            return;
        }
        let l = data.len() as u32;
        unsafe {
            match l {
                // fixext 1: special-case inline copy like C (no memcpy)
                1 => {
                    if let Some(p) = self.reserve(3) {
                        *p = 0xd4;
                        *p.add(1) = ext_type as u8;
                        *p.add(2) = data[0];
                    }
                    return;
                }
                2 | 4 | 8 | 16 => {
                    let tag: u8 = match l {
                        2 => 0xd5,
                        4 => 0xd6,
                        8 => 0xd7,
                        16 => 0xd8,
                        _ => unreachable!(),
                    };
                    if let Some(p) = self.reserve(l as usize + 2) {
                        *p = tag;
                        *p.add(1) = ext_type as u8;
                        ptr::copy_nonoverlapping(data.as_ptr(), p.add(2), l as usize);
                    }
                    return;
                }
                _ => {}
            }
            if l < 256 {
                if let Some(p) = self.reserve(l as usize + 3) {
                    *p = 0xc7;
                    *p.add(1) = l as u8;
                    *p.add(2) = ext_type as u8;
                    ptr::copy_nonoverlapping(data.as_ptr(), p.add(3), l as usize);
                }
                return;
            }
            if l < 65536 {
                if let Some(p) = self.reserve(l as usize + 4) {
                    *p = 0xc8;
                    let b = (l as u16).to_be_bytes();
                    *p.add(1) = b[0];
                    *p.add(2) = b[1];
                    *p.add(3) = ext_type as u8;
                    ptr::copy_nonoverlapping(data.as_ptr(), p.add(4), l as usize);
                }
                return;
            }
            if let Some(p) = self.reserve(l as usize + 6) {
                *p = 0xc9;
                ptr::copy_nonoverlapping(l.to_be_bytes().as_ptr(), p.add(1), 4);
                *p.add(5) = ext_type as u8;
                ptr::copy_nonoverlapping(data.as_ptr(), p.add(6), l as usize);
            }
        }
    }

    /// Encode a timestamp.
    /// - compat mode → ILLEGAL_CALL
    /// - nsec >= 1,000,000,000 → VALUE_ERROR
    /// - Selects timestamp32 / timestamp64 / timestamp96 based on range.
    ///
    /// Boundary logic (verified against C source):
    ///   timestamp32:  nsec==0 AND sec in [0, 2^32) AND (nsec<<34)|sec fits in 32 bits
    ///   timestamp64:  sec in [0, 2^34) AND above condition fails
    ///   timestamp96:  sec outside [0, 2^34) (negative OR >= 2^34)
    pub fn pack_time(&mut self, sec: i64, nsec: u32) {
        if self.return_code != RC_OK {
            return;
        }
        if self.be_compatible {
            self.return_code = RC_ILLEGAL_CALL;
            return;
        }
        if nsec >= 1_000_000_000 {
            self.return_code = RC_VALUE_ERROR;
            return;
        }
        // Cast sec to u64 and check bits 34–63 (matching C: (uint64_t)sec & 0xfffffffc00000000)
        let sec_u64 = sec as u64;
        unsafe {
            if sec_u64 & 0xffff_fffc_0000_0000u64 != 0 {
                // timestamp 96: 0xc7 0x0c 0xff | 4-byte nsec BE | 8-byte sec BE
                if let Some(p) = self.reserve(15) {
                    *p = 0xc7;
                    *p.add(1) = 12;
                    *p.add(2) = 0xff;
                    ptr::copy_nonoverlapping(nsec.to_be_bytes().as_ptr(), p.add(3), 4);
                    ptr::copy_nonoverlapping(sec.to_be_bytes().as_ptr(), p.add(7), 8);
                }
            } else {
                let data64: u64 = ((nsec as u64) << 34) | sec_u64;
                if data64 & 0xffff_ffff_0000_0000u64 != 0 {
                    // timestamp 64: 0xd7 0xff | 8-byte data64 BE
                    if let Some(p) = self.reserve(10) {
                        *p = 0xd7;
                        *p.add(1) = 0xff;
                        ptr::copy_nonoverlapping(data64.to_be_bytes().as_ptr(), p.add(2), 8);
                    }
                } else {
                    // timestamp 32: 0xd6 0xff | 4-byte data32 BE
                    let data32 = data64 as u32;
                    if let Some(p) = self.reserve(6) {
                        *p = 0xd6;
                        *p.add(1) = 0xff;
                        ptr::copy_nonoverlapping(data32.to_be_bytes().as_ptr(), p.add(2), 4);
                    }
                }
            }
        }
    }

    /// Raw insert of pre-encoded bytes.
    /// INTENTIONAL: no return_code check here — matches C cw_pack_insert exactly.
    /// See BUG-002 / DEV-002. This is an escape hatch for pre-encoded MsgPack blobs.
    pub fn pack_insert(&mut self, data: &[u8]) {
        // NO sticky-error check — intentional, matching C behaviour
        unsafe {
            if let Some(p) = self.reserve(data.len()) {
                ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
            }
        }
    }

    /// Flush the context. Calls flush_handler if set; else sets ILLEGAL_CALL.
    /// Only acts if return_code == RC_OK.
    pub fn pack_flush(&mut self) {
        if self.return_code != RC_OK {
            return;
        }
        match self.flush_handler {
            Some(handler) => {
                self.return_code = unsafe { handler(self as *mut PackContext) };
            }
            None => {
                self.return_code = RC_ILLEGAL_CALL;
            }
        }
    }
}
