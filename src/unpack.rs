//! CWPack unpacker — Rust port of cwpack.c unpack-side functions.
//!
//! Design decisions (see DECISIONS.md):
//! - `buffer_end_return_code` C macro pattern implemented via `mid_item: bool` param.
//! - BUG-001 (timestamp32 sign-extension) NOT reproduced: uses `tmpu32 as i64`
//!   instead of C's `(long)tmpu32`. See BUGS.md and DEV-001.
//! - `f64::from_bits()` used for double unpacking (safe, defined behavior).

use crate::types::{
    Blob, Container, Item, ItemValue, TimeSpec, ITEM_ARRAY, ITEM_BIN, ITEM_BOOLEAN, ITEM_DOUBLE,
    ITEM_EXT, ITEM_FLOAT, ITEM_MAP, ITEM_NEGATIVE_INTEGER, ITEM_NIL, ITEM_NOT_AN_ITEM,
    ITEM_POSITIVE_INTEGER, ITEM_STR, ITEM_TIMESTAMP, RC_BUFFER_UNDERFLOW, RC_END_OF_INPUT,
    RC_MALFORMED_INPUT, RC_OK, RC_WRONG_TIMESTAMP_LENGTH,
};
use std::ptr;

/// Underflow handler: given (context, bytes_needed), must refill the buffer.
/// Return RC_OK on success, or any CWP_RC_* error on failure.
/// If handler returns RC_END_OF_INPUT, it is remapped through the mid_item flag:
///   mid_item=false → RC_END_OF_INPUT, mid_item=true → RC_BUFFER_UNDERFLOW (DEV-003).
pub type UnpackUnderflowHandler = unsafe fn(*mut UnpackContext, usize) -> i32;

/// Mirrors `cw_unpack_context`.
pub struct UnpackContext {
    pub item: Item,
    pub start: *mut u8,
    pub current: *mut u8,
    pub end: *mut u8,
    pub return_code: i32,
    pub err_no: i32,
    pub underflow_handler: Option<UnpackUnderflowHandler>,
}

unsafe impl Send for UnpackContext {}

impl UnpackContext {
    /// Initialise from a byte slice. No underflow handler.
    pub fn new(buf: &[u8]) -> Self {
        let start = buf.as_ptr() as *mut u8;
        let end = unsafe { start.add(buf.len()) };
        UnpackContext {
            item: Item::default(),
            start,
            current: start,
            end,
            return_code: RC_OK,
            err_no: 0,
            underflow_handler: None,
        }
    }

    /// Initialise with an underflow handler.
    pub fn with_underflow_handler(buf: &[u8], handler: UnpackUnderflowHandler) -> Self {
        let mut ctx = Self::new(buf);
        ctx.underflow_handler = Some(handler);
        ctx
    }

    // -----------------------------------------------------------------------
    // Position/Cursor Helpers
    // -----------------------------------------------------------------------

    /// Number of bytes read so far.
    #[inline]
    pub fn current_pos(&self) -> usize {
        unsafe { self.current.offset_from(self.start) as usize }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Equivalent of `cw_unpack_assert_space` / `cw_unpack_assert_space_sub`.
    /// `mid_item=false` → END_OF_INPUT on underflow (reading first byte of item).
    /// `mid_item=true` → BUFFER_UNDERFLOW on underflow (already inside item).
    /// On success, advances `current` by `more` bytes and returns `Some(old_current)`.
    #[inline(always)]
    unsafe fn assert_space(&mut self, more: usize, mid_item: bool) -> Option<*mut u8> {
        let p = self.current;
        let nyp = p.add(more);
        if nyp <= self.end {
            self.current = nyp;
            return Some(p);
        }
        // Need more data
        let err = if mid_item {
            RC_BUFFER_UNDERFLOW
        } else {
            RC_END_OF_INPUT
        };
        match self.underflow_handler {
            None => {
                self.item.type_ = ITEM_NOT_AN_ITEM;
                self.return_code = err;
                None
            }
            Some(handler) => {
                let rc = handler(self as *mut UnpackContext, more);
                if rc != RC_OK {
                    // If handler signals END_OF_INPUT, remap through mid_item flag
                    let mapped = if rc == RC_END_OF_INPUT { err } else { rc };
                    self.item.type_ = ITEM_NOT_AN_ITEM;
                    self.return_code = mapped;
                    return None;
                }
                // Handler succeeded — re-read current
                let p2 = self.current;
                let nyp2 = p2.add(more);
                if nyp2 > self.end {
                    self.item.type_ = ITEM_NOT_AN_ITEM;
                    self.return_code = err;
                    return None;
                }
                self.current = nyp2;
                Some(p2)
            }
        }
    }

    /// Read 1 big-endian byte as u8.
    #[inline(always)]
    unsafe fn read1(&mut self) -> Option<u8> {
        let p = self.assert_space(1, true)?;
        Some(*p)
    }

    /// Read 2 big-endian bytes as u16.
    #[inline(always)]
    unsafe fn read2(&mut self) -> Option<u16> {
        let p = self.assert_space(2, true)?;
        Some(u16::from_be_bytes([*p, *p.add(1)]))
    }

    /// Read 4 big-endian bytes as u32.
    #[inline(always)]
    unsafe fn read4(&mut self) -> Option<u32> {
        let p = self.assert_space(4, true)?;
        Some(u32::from_be_bytes([*p, *p.add(1), *p.add(2), *p.add(3)]))
    }

    /// Read 8 big-endian bytes as u64.
    #[inline(always)]
    unsafe fn read8(&mut self) -> Option<u64> {
        let p = self.assert_space(8, true)?;
        let mut b = [0u8; 8];
        ptr::copy_nonoverlapping(p, b.as_mut_ptr(), 8);
        Some(u64::from_be_bytes(b))
    }

    /// Set item type + blob (str/bin/ext) after reading length.
    /// Advances current by `length` bytes, sets blob.start.
    #[inline(always)]
    unsafe fn assert_blob(&mut self, length: u32) -> Option<*const u8> {
        let p = self.assert_space(length as usize, true)?;
        Some(p as *const u8)
    }

    /// getDDItemFix — handles fixext 1/2/4/8/16.
    /// Reads type byte; dispatches timestamp32/64 or sets ext blob.
    /// Returns false if an error was set.
    unsafe fn read_fixext(&mut self, len: u32) -> bool {
        // assert_space(len+1) already consumed by caller reading format byte;
        // we need len more bytes starting from current
        // Actually: getDDItemFix reads len+1 bytes (type byte + data)
        let type_p = match self.assert_space(len as usize + 1, true) {
            None => return false,
            Some(p) => p,
        };
        let ext_type = *(type_p as *const i8) as i32;
        let data_p = type_p.add(1);

        self.item.type_ = ext_type;
        if ext_type == ITEM_TIMESTAMP {
            if len == 4 {
                // timestamp32: 4 bytes = seconds as u32
                // BUG-001 FIX: use u32 as i64 (zero-extend), NOT (long)tmpu32
                let sec_u32 = u32::from_be_bytes([
                    *data_p,
                    *data_p.add(1),
                    *data_p.add(2),
                    *data_p.add(3),
                ]);
                self.item.as_.time = TimeSpec {
                    tv_sec: sec_u32 as i64, // zero-extension, always correct (C has BUG-001 here)
                    tv_nsec: 0,
                };
                return true;
            } else if len == 8 {
                // timestamp64: 8 bytes = (nsec:30 | sec:34)
                let mut b = [0u8; 8];
                ptr::copy_nonoverlapping(data_p, b.as_mut_ptr(), 8);
                let raw = u64::from_be_bytes(b);
                self.item.as_.time = TimeSpec {
                    tv_sec: (raw & 0x0000_0003_ffff_ffffu64) as i64,
                    tv_nsec: (raw >> 34) as u32,
                };
                return true;
            } else {
                // Any other fixext length with type==-1 → WRONG_TIMESTAMP_LENGTH
                self.item.type_ = ITEM_NOT_AN_ITEM;
                self.return_code = RC_WRONG_TIMESTAMP_LENGTH;
                return false;
            }
        }
        // Non-timestamp ext: set blob pointing into buffer
        self.item.as_.ext = Blob {
            start: data_p as *const u8,
            length: len,
        };
        true
    }

    // -----------------------------------------------------------------------
    // Public unpack functions
    // -----------------------------------------------------------------------

    /// Decode the next item from the buffer.
    /// Sticky-error no-op. Sets `self.item` on success.
    /// On end of stream (first-byte read fails) → RC_END_OF_INPUT.
    /// On mid-item failure → RC_BUFFER_UNDERFLOW.
    pub fn unpack_next(&mut self) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe {
            // Read first byte — use mid_item=false → END_OF_INPUT on underflow
            let first_p = match self.assert_space(1, false) {
                None => return,
                Some(p) => p,
            };
            let c = *first_p;

            // From here on: mid_item=true → BUFFER_UNDERFLOW on underflow
            match c {
                // positive fixint 0x00–0x7f
                0x00..=0x7f => {
                    self.item = Item {
                        type_: ITEM_POSITIVE_INTEGER,
                        as_: ItemValue { i64_: c as i64 },
                    };
                }
                // fixmap 0x80–0x8f
                0x80..=0x8f => {
                    self.item = Item {
                        type_: ITEM_MAP,
                        as_: ItemValue {
                            map: Container { size: (c & 0x0f) as u32 },
                        },
                    };
                }
                // fixarray 0x90–0x9f
                0x90..=0x9f => {
                    self.item = Item {
                        type_: ITEM_ARRAY,
                        as_: ItemValue {
                            array: Container { size: (c & 0x0f) as u32 },
                        },
                    };
                }
                // fixstr 0xa0–0xbf
                0xa0..=0xbf => {
                    let len = (c & 0x1f) as u32;
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ITEM_STR,
                                as_: ItemValue { str_: Blob { start, length: len } },
                            };
                        }
                    }
                }
                0xc0 => {
                    self.item.type_ = ITEM_NIL;
                }
                // 0xc1 unused → MALFORMED_INPUT (same as default)
                0xc2 => {
                    self.item = Item {
                        type_: ITEM_BOOLEAN,
                        as_: ItemValue { boolean: false },
                    };
                }
                0xc3 => {
                    self.item = Item {
                        type_: ITEM_BOOLEAN,
                        as_: ItemValue { boolean: true },
                    };
                }
                // bin 8
                0xc4 => {
                    let len = match self.read1() { None => return, Some(v) => v as u32 };
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ITEM_BIN,
                                as_: ItemValue { bin: Blob { start, length: len } },
                            };
                        }
                    }
                }
                // bin 16
                0xc5 => {
                    let len = match self.read2() { None => return, Some(v) => v as u32 };
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ITEM_BIN,
                                as_: ItemValue { bin: Blob { start, length: len } },
                            };
                        }
                    }
                }
                // bin 32
                0xc6 => {
                    let len = match self.read4() { None => return, Some(v) => v };
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ITEM_BIN,
                                as_: ItemValue { bin: Blob { start, length: len } },
                            };
                        }
                    }
                }
                // ext 8 — only format that can carry timestamp96
                0xc7 => {
                    let len = match self.read1() { None => return, Some(v) => v as u32 };
                    let type_p = match self.assert_space(1, true) {
                        None => return,
                        Some(p) => p,
                    };
                    let ext_type = *(type_p as *const i8) as i32;
                    if ext_type == ITEM_TIMESTAMP {
                        if len == 12 {
                            let nsec = match self.read4() { None => return, Some(v) => v };
                            let sec_raw = match self.read8() { None => return, Some(v) => v };
                            self.item = Item {
                                type_: ITEM_TIMESTAMP,
                                as_: ItemValue {
                                    time: TimeSpec {
                                        tv_sec: sec_raw as i64,
                                        tv_nsec: nsec,
                                    },
                                },
                            };
                            return;
                        }
                        self.item.type_ = ITEM_NOT_AN_ITEM;
                        self.return_code = RC_WRONG_TIMESTAMP_LENGTH;
                        return;
                    }
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ext_type,
                                as_: ItemValue { ext: Blob { start, length: len } },
                            };
                        }
                    }
                }
                // ext 16 (no timestamp dispatch for this format)
                0xc8 => {
                    let len = match self.read2() { None => return, Some(v) => v as u32 };
                    let type_p = match self.assert_space(1, true) {
                        None => return,
                        Some(p) => p,
                    };
                    let ext_type = *(type_p as *const i8) as i32;
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ext_type,
                                as_: ItemValue { ext: Blob { start, length: len } },
                            };
                        }
                    }
                }
                // ext 32
                0xc9 => {
                    let len = match self.read4() { None => return, Some(v) => v };
                    let type_p = match self.assert_space(1, true) {
                        None => return,
                        Some(p) => p,
                    };
                    let ext_type = *(type_p as *const i8) as i32;
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ext_type,
                                as_: ItemValue { ext: Blob { start, length: len } },
                            };
                        }
                    }
                }
                // float32
                0xca => {
                    let bits = match self.read4() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_FLOAT,
                        as_: ItemValue { real: f32::from_bits(bits) },
                    };
                }
                // float64 — getDDItem8 stores to u64 in C; we use long_real directly
                0xcb => {
                    let bits = match self.read8() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_DOUBLE,
                        as_: ItemValue { long_real: f64::from_bits(bits) },
                    };
                }
                // uint8
                0xcc => {
                    let v = match self.read1() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_POSITIVE_INTEGER,
                        as_: ItemValue { u64_: v as u64 },
                    };
                }
                // uint16
                0xcd => {
                    let v = match self.read2() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_POSITIVE_INTEGER,
                        as_: ItemValue { u64_: v as u64 },
                    };
                }
                // uint32
                0xce => {
                    let v = match self.read4() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_POSITIVE_INTEGER,
                        as_: ItemValue { u64_: v as u64 },
                    };
                }
                // uint64
                0xcf => {
                    let v = match self.read8() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_POSITIVE_INTEGER,
                        as_: ItemValue { u64_: v },
                    };
                }
                // int8: reclassify to POSITIVE if value >= 0
                0xd0 => {
                    let v = match self.read1() { None => return, Some(v) => v as i8 as i64 };
                    self.item = Item {
                        type_: if v >= 0 { ITEM_POSITIVE_INTEGER } else { ITEM_NEGATIVE_INTEGER },
                        as_: ItemValue { i64_: v },
                    };
                }
                // int16
                0xd1 => {
                    let v = match self.read2() { None => return, Some(v) => v as i16 as i64 };
                    self.item = Item {
                        type_: if v >= 0 { ITEM_POSITIVE_INTEGER } else { ITEM_NEGATIVE_INTEGER },
                        as_: ItemValue { i64_: v },
                    };
                }
                // int32
                0xd2 => {
                    let v = match self.read4() { None => return, Some(v) => v as i32 as i64 };
                    self.item = Item {
                        type_: if v >= 0 { ITEM_POSITIVE_INTEGER } else { ITEM_NEGATIVE_INTEGER },
                        as_: ItemValue { i64_: v },
                    };
                }
                // int64
                0xd3 => {
                    let v = match self.read8() { None => return, Some(v) => v as i64 };
                    self.item = Item {
                        type_: if v >= 0 { ITEM_POSITIVE_INTEGER } else { ITEM_NEGATIVE_INTEGER },
                        as_: ItemValue { i64_: v },
                    };
                }
                // fixext 1
                0xd4 => { self.read_fixext(1); }
                // fixext 2
                0xd5 => { self.read_fixext(2); }
                // fixext 4
                0xd6 => { self.read_fixext(4); }
                // fixext 8
                0xd7 => { self.read_fixext(8); }
                // fixext 16
                0xd8 => { self.read_fixext(16); }
                // str8
                0xd9 => {
                    let len = match self.read1() { None => return, Some(v) => v as u32 };
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ITEM_STR,
                                as_: ItemValue { str_: Blob { start, length: len } },
                            };
                        }
                    }
                }
                // str16
                0xda => {
                    let len = match self.read2() { None => return, Some(v) => v as u32 };
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ITEM_STR,
                                as_: ItemValue { str_: Blob { start, length: len } },
                            };
                        }
                    }
                }
                // str32
                0xdb => {
                    let len = match self.read4() { None => return, Some(v) => v };
                    match self.assert_blob(len) {
                        None => {}
                        Some(start) => {
                            self.item = Item {
                                type_: ITEM_STR,
                                as_: ItemValue { str_: Blob { start, length: len } },
                            };
                        }
                    }
                }
                // array16
                0xdc => {
                    let v = match self.read2() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_ARRAY,
                        as_: ItemValue { array: Container { size: v as u32 } },
                    };
                }
                // array32
                0xdd => {
                    let v = match self.read4() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_ARRAY,
                        as_: ItemValue { array: Container { size: v } },
                    };
                }
                // map16
                0xde => {
                    let v = match self.read2() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_MAP,
                        as_: ItemValue { map: Container { size: v as u32 } },
                    };
                }
                // map32
                0xdf => {
                    let v = match self.read4() { None => return, Some(v) => v };
                    self.item = Item {
                        type_: ITEM_MAP,
                        as_: ItemValue { map: Container { size: v } },
                    };
                }
                // negative fixint 0xe0–0xff
                0xe0..=0xff => {
                    self.item = Item {
                        type_: ITEM_NEGATIVE_INTEGER,
                        as_: ItemValue { i64_: c as i8 as i64 },
                    };
                }
                // 0xc1 and any other undefined byte
                _ => {
                    self.item.type_ = ITEM_NOT_AN_ITEM;
                    self.return_code = RC_MALFORMED_INPUT;
                }
            }
        }
    }

    /// Skip `item_count` top-level items. Recurses into containers via counter
    /// accumulation (no recursion — matches C cw_skip_items exactly).
    /// Sticky-error no-op.
    pub fn skip_items(&mut self, mut item_count: i64) {
        if self.return_code != RC_OK {
            return;
        }
        unsafe {
            while item_count > 0 {
                item_count -= 1;

                // Read format byte — mid_item=false (at item boundary)
                let first_p = match self.assert_space(1, false) {
                    None => return,
                    Some(p) => p,
                };
                let c = *first_p;

                // Switch on format byte — mirrors C cw_skip_items switch with fallthrough
                match c {
                    // Fixed-size 1-byte items: fixint+/-, nil, false, true
                    0x00..=0x7f | 0xe0..=0xff | 0xc0 | 0xc2 | 0xc3 => {
                        // Nothing extra to skip
                    }
                    // uint8, int8: skip 1 byte
                    0xcc | 0xd0 => {
                        if self.assert_space(1, true).is_none() { return; }
                    }
                    // uint16, int16, fixext1: skip 2 bytes
                    0xcd | 0xd1 | 0xd4 => {
                        if self.assert_space(2, true).is_none() { return; }
                    }
                    // fixext2: skip 3 bytes
                    0xd5 => {
                        if self.assert_space(3, true).is_none() { return; }
                    }
                    // float32, uint32, int32: skip 4 bytes
                    0xca | 0xce | 0xd2 => {
                        if self.assert_space(4, true).is_none() { return; }
                    }
                    // fixext4: skip 5 bytes
                    0xd6 => {
                        if self.assert_space(5, true).is_none() { return; }
                    }
                    // float64, uint64, int64: skip 8 bytes
                    0xcb | 0xcf | 0xd3 => {
                        if self.assert_space(8, true).is_none() { return; }
                    }
                    // fixext8: skip 9 bytes
                    0xd7 => {
                        if self.assert_space(9, true).is_none() { return; }
                    }
                    // fixext16: skip 17 bytes
                    0xd8 => {
                        if self.assert_space(17, true).is_none() { return; }
                    }
                    // fixstr 0xa0–0xbf: skip (c & 0x1f) bytes
                    0xa0..=0xbf => {
                        let len = (c & 0x1f) as usize;
                        if self.assert_space(len, true).is_none() { return; }
                    }
                    // str8, bin8: read 1-byte length then skip
                    0xd9 | 0xc4 => {
                        let p = match self.assert_space(1, true) { None => return, Some(p) => p };
                        let len = *p as usize;
                        if self.assert_space(len, true).is_none() { return; }
                    }
                    // str16, bin16: read 2-byte length then skip
                    0xda | 0xc5 => {
                        let len = match self.read2() { None => return, Some(v) => v as usize };
                        if self.assert_space(len, true).is_none() { return; }
                    }
                    // str32, bin32: read 4-byte length then skip
                    0xdb | 0xc6 => {
                        let len = match self.read4() { None => return, Some(v) => v as usize };
                        if self.assert_space(len, true).is_none() { return; }
                    }
                    // fixmap 0x80–0x8f: accumulate 2*(c & 0xf) more items
                    0x80..=0x8f => {
                        item_count += 2 * (c & 0x0f) as i64;
                    }
                    // fixarray 0x90–0x9f: accumulate (c & 0xf) more items
                    0x90..=0x9f => {
                        item_count += (c & 0x0f) as i64;
                    }
                    // array16
                    0xdc => {
                        let n = match self.read2() { None => return, Some(v) => v as i64 };
                        item_count += n;
                    }
                    // map16
                    0xde => {
                        let n = match self.read2() { None => return, Some(v) => v as i64 };
                        item_count += 2 * n;
                    }
                    // array32
                    0xdd => {
                        let n = match self.read4() { None => return, Some(v) => v as i64 };
                        item_count += n;
                    }
                    // map32
                    0xdf => {
                        let n = match self.read4() { None => return, Some(v) => v as i64 };
                        item_count += 2 * n;
                    }
                    // ext8: read 1-byte length, skip length+1 bytes (type byte included)
                    0xc7 => {
                        let p = match self.assert_space(1, true) { None => return, Some(p) => p };
                        let len = *p as usize;
                        if self.assert_space(len + 1, true).is_none() { return; }
                    }
                    // ext16: read 2-byte length, skip length+1
                    0xc8 => {
                        let len = match self.read2() { None => return, Some(v) => v as usize };
                        if self.assert_space(len + 1, true).is_none() { return; }
                    }
                    // ext32: read 4-byte length, skip length+1
                    0xc9 => {
                        let len = match self.read4() { None => return, Some(v) => v as usize };
                        if self.assert_space(len + 1, true).is_none() { return; }
                    }
                    _ => {
                        self.item.type_ = ITEM_NOT_AN_ITEM;
                        self.return_code = RC_MALFORMED_INPUT;
                        return;
                    }
                }
            }
        }
    }

    /// Peek at the type of the next item without consuming it.
    /// Returns ITEM_NOT_AN_ITEM if already errored or on underflow.
    /// For ext types, peeks ahead to read the type byte.
    pub fn look_ahead(&mut self) -> i32 {
        if self.return_code != RC_OK {
            return ITEM_NOT_AN_ITEM;
        }
        unsafe {
            // Peek first byte without permanent advance — mid_item=false
            let first_p = match self.assert_space(1, false) {
                None => return ITEM_NOT_AN_ITEM,
                Some(p) => p,
            };
            // Step back immediately — we're peeking only
            self.current = self.current.sub(1);
            let c = *first_p;

            match c {
                0x00..=0x7f => ITEM_POSITIVE_INTEGER,
                0x80..=0x8f => ITEM_MAP,
                0x90..=0x9f => ITEM_ARRAY,
                0xa0..=0xbf => ITEM_STR,
                0xc0 => ITEM_NIL,
                0xc2 | 0xc3 => ITEM_BOOLEAN,
                0xc4 | 0xc5 | 0xc6 => ITEM_BIN,
                // ext8: peek 3 bytes (format + length + type), step all back
                0xc7 => {
                    match self.assert_space(3, true) {
                        None => ITEM_NOT_AN_ITEM,
                        Some(p) => {
                            self.current = self.current.sub(3);
                            let type_byte = *(p.add(2) as *const i8) as i32;
                            if type_byte == ITEM_TIMESTAMP { ITEM_TIMESTAMP } else { type_byte }
                        }
                    }
                }
                // ext16: peek 4 bytes
                0xc8 => {
                    match self.assert_space(4, true) {
                        None => ITEM_NOT_AN_ITEM,
                        Some(p) => {
                            self.current = self.current.sub(4);
                            *(p.add(3) as *const i8) as i32
                        }
                    }
                }
                // ext32: peek 6 bytes
                0xc9 => {
                    match self.assert_space(6, true) {
                        None => ITEM_NOT_AN_ITEM,
                        Some(p) => {
                            self.current = self.current.sub(6);
                            *(p.add(5) as *const i8) as i32
                        }
                    }
                }
                0xca => ITEM_FLOAT,
                0xcb => ITEM_DOUBLE,
                0xcc | 0xcd | 0xce | 0xcf => ITEM_POSITIVE_INTEGER,
                0xd0 | 0xd1 | 0xd2 | 0xd3 => ITEM_NEGATIVE_INTEGER,
                // fixext 1/2/4/8/16: peek 2 bytes (format + type), step back
                0xd4 | 0xd5 | 0xd6 | 0xd7 | 0xd8 => {
                    match self.assert_space(2, true) {
                        None => ITEM_NOT_AN_ITEM,
                        Some(p) => {
                            self.current = self.current.sub(2);
                            *(p.add(1) as *const i8) as i32
                        }
                    }
                }
                0xd9 | 0xda | 0xdb => ITEM_STR,
                0xdc | 0xdd => ITEM_ARRAY,
                0xde | 0xdf => ITEM_MAP,
                0xe0..=0xff => ITEM_NEGATIVE_INTEGER,
                _ => ITEM_NOT_AN_ITEM,
            }
        }
    }
}


