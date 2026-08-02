/// CWPack return codes — mirrors C `CWP_RC_*` constants exactly.
pub const RC_OK: i32 = 0;
pub const RC_END_OF_INPUT: i32 = -1;
pub const RC_BUFFER_OVERFLOW: i32 = -2;
pub const RC_BUFFER_UNDERFLOW: i32 = -3;
pub const RC_MALFORMED_INPUT: i32 = -4;
pub const RC_WRONG_BYTE_ORDER: i32 = -5;
pub const RC_ERROR_IN_HANDLER: i32 = -6;
pub const RC_ILLEGAL_CALL: i32 = -7;
pub const RC_MALLOC_ERROR: i32 = -8;
pub const RC_STOPPED: i32 = -9;
pub const RC_TYPE_ERROR: i32 = -10;
pub const RC_VALUE_ERROR: i32 = -11;
pub const RC_WRONG_TIMESTAMP_LENGTH: i32 = -12;

/// CWPack item type codes — mirrors C `cwpack_item_types` enum exactly.
/// Values -128..=-1 are reserved/system ext types; -1 = TIMESTAMP.
/// Values 0..=127 are user ext types (numeric value = ext type ID).
/// Values 300..=310 are standard MsgPack value types.
/// 999 = NOT_AN_ITEM (error sentinel).
pub const ITEM_MIN_RESERVED_EXT: i32 = -128;
pub const ITEM_TIMESTAMP: i32 = -1;
pub const ITEM_MAX_RESERVED_EXT: i32 = -1;
pub const ITEM_MIN_USER_EXT: i32 = 0;
pub const ITEM_MAX_USER_EXT: i32 = 127;

pub const ITEM_NIL: i32 = 300;
pub const ITEM_BOOLEAN: i32 = 301;
pub const ITEM_POSITIVE_INTEGER: i32 = 302;
pub const ITEM_NEGATIVE_INTEGER: i32 = 303;
pub const ITEM_FLOAT: i32 = 304;
pub const ITEM_DOUBLE: i32 = 305;
pub const ITEM_STR: i32 = 306;
pub const ITEM_BIN: i32 = 307;
pub const ITEM_ARRAY: i32 = 308;
pub const ITEM_MAP: i32 = 309;
pub const ITEM_EXT: i32 = 310;
pub const ITEM_NOT_AN_ITEM: i32 = 999;

// ---------------------------------------------------------------------------
// Shared data types used by both pack (cw_pack_str etc.) and unpack contexts
// ---------------------------------------------------------------------------

/// Mirrors `cwpack_blob` — zero-copy pointer into the input buffer.
/// `start` is valid for the lifetime of the unpack context's buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Blob {
    pub start: *const u8,
    pub length: u32,
}

unsafe impl Send for Blob {}

/// Mirrors `cwpack_container` — array or map size.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Container {
    pub size: u32,
}

/// Mirrors `cwpack_timespec`.
/// NOTE: tv_sec is int64_t. The Rust port always zero-extends u32 timestamps
/// correctly (unlike the C original which has BUG-001 on LLP64 platforms).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TimeSpec {
    pub tv_sec: i64,
    pub tv_nsec: u32,
}

/// Mirrors the `item.as` union in `cwpack_item`.
/// Access the correct field based on `item.type_`.
/// Using C union semantics: all fields share the same memory.
#[repr(C)]
#[derive(Clone, Copy)]
pub union ItemValue {
    pub boolean: bool,
    pub u64_: u64,
    pub i64_: i64,
    pub real: f32,
    pub long_real: f64,
    pub array: Container,
    pub map: Container,
    pub str_: Blob,
    pub bin: Blob,
    pub ext: Blob,
    pub time: TimeSpec,
}

/// Mirrors `cwpack_item` — the decoded item from one call to `cw_unpack_next`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Item {
    pub type_: i32,
    pub as_: ItemValue,
}

impl Default for Item {
    fn default() -> Self {
        Item {
            type_: ITEM_NOT_AN_ITEM,
            as_: ItemValue { u64_: 0 },
        }
    }
}
