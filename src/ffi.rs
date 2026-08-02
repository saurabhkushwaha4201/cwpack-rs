#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_long, c_ulong};

// ---------------------------------------------------------------------------
// Return codes — mirrors C CWP_RC_* constants
// ---------------------------------------------------------------------------
pub const CWP_RC_OK: c_int = 0;
pub const CWP_RC_END_OF_INPUT: c_int = -1;
pub const CWP_RC_BUFFER_OVERFLOW: c_int = -2;
pub const CWP_RC_BUFFER_UNDERFLOW: c_int = -3;
pub const CWP_RC_MALFORMED_INPUT: c_int = -4;
pub const CWP_RC_WRONG_BYTE_ORDER: c_int = -5;
pub const CWP_RC_ERROR_IN_HANDLER: c_int = -6;
pub const CWP_RC_ILLEGAL_CALL: c_int = -7;
pub const CWP_RC_MALLOC_ERROR: c_int = -8;
pub const CWP_RC_STOPPED: c_int = -9;
pub const CWP_RC_TYPE_ERROR: c_int = -10;
pub const CWP_RC_VALUE_ERROR: c_int = -11;
pub const CWP_RC_WRONG_TIMESTAMP_LENGTH: c_int = -12;

// ---------------------------------------------------------------------------
// Item type constants — mirrors C cwpack_item_types enum
// ---------------------------------------------------------------------------
pub const CWP_ITEM_TIMESTAMP: c_int = -1;
pub const CWP_ITEM_NIL: c_int = 300;
pub const CWP_ITEM_BOOLEAN: c_int = 301;
pub const CWP_ITEM_POSITIVE_INTEGER: c_int = 302;
pub const CWP_ITEM_NEGATIVE_INTEGER: c_int = 303;
pub const CWP_ITEM_FLOAT: c_int = 304;
pub const CWP_ITEM_DOUBLE: c_int = 305;
pub const CWP_ITEM_STR: c_int = 306;
pub const CWP_ITEM_BIN: c_int = 307;
pub const CWP_ITEM_ARRAY: c_int = 308;
pub const CWP_ITEM_MAP: c_int = 309;
pub const CWP_ITEM_EXT: c_int = 310;
pub const CWP_NOT_AN_ITEM: c_int = 999;

// ---------------------------------------------------------------------------
// Pack context struct — mirrors C cw_pack_context exactly (verified layout)
// ---------------------------------------------------------------------------
#[repr(C)]
pub struct CwPackContext {
    pub current: *mut u8,
    pub start: *mut u8,
    pub end: *mut u8,
    pub be_compatible: bool,
    pub return_code: c_int,
    pub err_no: c_int,
    pub handle_pack_overflow:
        Option<unsafe extern "C" fn(*mut CwPackContext, c_ulong) -> c_int>,
    pub handle_flush: Option<unsafe extern "C" fn(*mut CwPackContext) -> c_int>,
}

// ---------------------------------------------------------------------------
// Unpack context supporting types
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Copy, Clone)]
pub struct CwPackBlob {
    pub start: *const c_void,
    pub length: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CwPackContainer {
    pub size: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CwPackTimeSpec {
    pub tv_sec: i64,
    pub tv_nsec: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union CwPackItemUnion {
    pub boolean: bool,
    pub u64_: u64,
    pub i64_: i64,
    pub real: f32,
    pub long_real: f64,
    pub array: CwPackContainer,
    pub map: CwPackContainer,
    pub str_: CwPackBlob,
    pub bin: CwPackBlob,
    pub ext: CwPackBlob,
    pub time: CwPackTimeSpec,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CwPackItem {
    pub type_: c_int,
    pub as_: CwPackItemUnion,
}

// ---------------------------------------------------------------------------
// Unpack context struct — mirrors C cw_unpack_context exactly (verified layout)
// ---------------------------------------------------------------------------
#[repr(C)]
pub struct CwUnpackContext {
    pub item: CwPackItem,
    pub start: *mut u8,
    pub current: *mut u8,
    pub end: *mut u8, // logical end of buffer
    pub return_code: c_int,
    pub err_no: c_int,
    pub handle_unpack_underflow:
        Option<unsafe extern "C" fn(*mut CwUnpackContext, c_ulong) -> c_int>,
}

// ---------------------------------------------------------------------------
// Extern C declarations — all 19 exported CWPack functions
// ---------------------------------------------------------------------------
extern "C" {
    // --- Pack context lifecycle ---
    pub fn cw_pack_context_init(
        pack_context: *mut CwPackContext,
        data: *mut c_void,
        length: c_ulong,
        hpo: Option<unsafe extern "C" fn(*mut CwPackContext, c_ulong) -> c_int>,
    ) -> c_int;

    pub fn cw_pack_set_compatibility(pack_context: *mut CwPackContext, be_compatible: bool);

    pub fn cw_pack_set_flush_handler(
        pack_context: *mut CwPackContext,
        handle_flush: Option<unsafe extern "C" fn(*mut CwPackContext) -> c_int>,
    );

    pub fn cw_pack_flush(pack_context: *mut CwPackContext);

    // --- Pack scalar values ---
    pub fn cw_pack_nil(pack_context: *mut CwPackContext);
    pub fn cw_pack_true(pack_context: *mut CwPackContext);
    pub fn cw_pack_false(pack_context: *mut CwPackContext);
    pub fn cw_pack_boolean(pack_context: *mut CwPackContext, b: bool);

    pub fn cw_pack_signed(pack_context: *mut CwPackContext, i: i64);
    pub fn cw_pack_unsigned(pack_context: *mut CwPackContext, i: u64);

    pub fn cw_pack_float(pack_context: *mut CwPackContext, f: f32);
    pub fn cw_pack_double(pack_context: *mut CwPackContext, d: f64);

    // --- Pack container sizes ---
    pub fn cw_pack_array_size(pack_context: *mut CwPackContext, n: u32);
    pub fn cw_pack_map_size(pack_context: *mut CwPackContext, n: u32);

    // --- Pack variable-length values ---
    pub fn cw_pack_str(pack_context: *mut CwPackContext, v: *const c_char, l: u32);
    pub fn cw_pack_bin(pack_context: *mut CwPackContext, v: *const c_void, l: u32);
    pub fn cw_pack_ext(pack_context: *mut CwPackContext, type_: i8, v: *const c_void, l: u32);
    pub fn cw_pack_time(pack_context: *mut CwPackContext, sec: i64, nsec: u32);

    /// Raw insert — no sticky-error guard (BUG-002 / DEV-002: intentional bypass).
    pub fn cw_pack_insert(pack_context: *mut CwPackContext, v: *const c_void, l: u32);

    // --- Unpack context lifecycle ---
    pub fn cw_unpack_context_init(
        unpack_context: *mut CwUnpackContext,
        data: *const c_void,
        length: c_ulong,
        huu: Option<unsafe extern "C" fn(*mut CwUnpackContext, c_ulong) -> c_int>,
    ) -> c_int;

    // --- Unpack operations ---
    pub fn cw_unpack_next(unpack_context: *mut CwUnpackContext);
    pub fn cw_skip_items(unpack_context: *mut CwUnpackContext, item_count: c_long);
    pub fn cw_look_ahead(unpack_context: *mut CwUnpackContext) -> c_int;
}
