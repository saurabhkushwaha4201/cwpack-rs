//! Differential tests: integer encoding boundaries (pack only — comparing byte output).
//! Verifies all fixint/intN/uintN width transitions in both pack_signed and pack_unsigned.
//! Also verifies REQ-FIX-002: signed 0–127 → fixint, NOT uint8.

#[cfg(test)]
mod int_boundaries {
    use cwpack_rs::ffi::{self, CwPackContext, CWP_RC_OK};
    use cwpack_rs::pack::PackContext;
    use cwpack_rs::types::RC_OK;
    use std::ffi::c_void;

    unsafe fn c_pack_unsigned(v: u64) -> Vec<u8> {
        let mut buf = [0u8; 16];
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(ctx.as_mut_ptr(), buf.as_mut_ptr() as *mut c_void, 16 as _, None);
        let mut ctx = ctx.assume_init();
        ffi::cw_pack_unsigned(&mut ctx, v);
        assert_eq!(ctx.return_code, CWP_RC_OK);
        let n = ctx.current.offset_from(ctx.start) as usize;
        buf[..n].to_vec()
    }

    unsafe fn c_pack_signed(v: i64) -> Vec<u8> {
        let mut buf = [0u8; 16];
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(ctx.as_mut_ptr(), buf.as_mut_ptr() as *mut c_void, 16 as _, None);
        let mut ctx = ctx.assume_init();
        ffi::cw_pack_signed(&mut ctx, v);
        assert_eq!(ctx.return_code, CWP_RC_OK);
        let n = ctx.current.offset_from(ctx.start) as usize;
        buf[..n].to_vec()
    }

    fn rust_pack_unsigned(v: u64) -> (Vec<u8>, i32) {
        let mut buf = [0u8; 16];
        let mut ctx = PackContext::new(&mut buf);
        ctx.pack_unsigned(v);
        let rc = ctx.return_code;
        let n = ctx.written();
        (buf[..n].to_vec(), rc)
    }

    fn rust_pack_signed(v: i64) -> (Vec<u8>, i32) {
        let mut buf = [0u8; 16];
        let mut ctx = PackContext::new(&mut buf);
        ctx.pack_signed(v);
        let rc = ctx.return_code;
        let n = ctx.written();
        (buf[..n].to_vec(), rc)
    }

    macro_rules! diff_unsigned {
        ($name:ident, $v:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let c_out = unsafe { c_pack_unsigned($v) };
                let (rs_out, rs_rc) = rust_pack_unsigned($v);
                assert_eq!(rs_rc, RC_OK);
                assert_eq!(c_out, rs_out, "unsigned({}) C vs Rust mismatch", $v);
                assert_eq!(rs_out, $expected, "unsigned({}) wrong encoding", $v);
            }
        };
    }

    macro_rules! diff_signed {
        ($name:ident, $v:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let c_out = unsafe { c_pack_signed($v) };
                let (rs_out, rs_rc) = rust_pack_signed($v);
                assert_eq!(rs_rc, RC_OK);
                assert_eq!(c_out, rs_out, "signed({}) C vs Rust mismatch", $v);
                assert_eq!(rs_out, $expected, "signed({}) wrong encoding", $v);
            }
        };
    }

    // --- unsigned int boundaries ---
    diff_unsigned!(uint_0,   0,   &[0x00u8]);
    diff_unsigned!(uint_127, 127, &[0x7fu8]);          // fixuint max
    diff_unsigned!(uint_128, 128, &[0xccu8, 0x80]);    // uint8 min
    diff_unsigned!(uint_255, 255, &[0xccu8, 0xff]);    // uint8 max
    diff_unsigned!(uint_256, 256, &[0xcdu8, 0x01, 0x00]); // uint16 min
    diff_unsigned!(uint_65535, 65535, &[0xcdu8, 0xff, 0xff]); // uint16 max
    diff_unsigned!(uint_65536, 65536, &[0xceu8, 0x00, 0x01, 0x00, 0x00]); // uint32 min
    diff_unsigned!(uint_max_u32, 0xffff_ffffu64, &[0xceu8, 0xff, 0xff, 0xff, 0xff]); // uint32 max
    diff_unsigned!(uint_u64_min, 0x1_0000_0000u64, &[0xcfu8, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]); // uint64 min
    diff_unsigned!(uint_max_u64, u64::MAX, &[0xcfu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // uint64 max

    // --- signed int boundaries ---
    // REQ-FIX-002: 0 and 127 must encode as fixint (single byte), NOT as 0xcc
    diff_signed!(sint_0,    0i64,    &[0x00u8]);  // fixint (NOT uint8!)
    diff_signed!(sint_127,  127i64,  &[0x7fu8]);  // fixint max (NOT uint8!)
    diff_signed!(sint_128,  128i64,  &[0xccu8, 0x80]); // uint8 (positive > 127 uses uint encoding)
    diff_signed!(sint_neg1, -1i64,   &[0xffu8]);  // negative fixint
    diff_signed!(sint_neg32, -32i64, &[0xe0u8]); // negative fixint min
    diff_signed!(sint_neg33, -33i64, &[0xd0u8, 0xdf]); // int8
    diff_signed!(sint_neg128, -128i64, &[0xd0u8, 0x80]); // int8 min
    diff_signed!(sint_neg129, -129i64, &[0xd1u8, 0xff, 0x7f]); // int16
    diff_signed!(sint_neg32768, -32768i64, &[0xd1u8, 0x80, 0x00]); // int16 min
    diff_signed!(sint_neg32769, -32769i64, &[0xd2u8, 0xff, 0xff, 0x7f, 0xff]); // int32
    diff_signed!(sint_i32_min, i32::MIN as i64, &[0xd2u8, 0x80, 0x00, 0x00, 0x00]); // int32 min
    diff_signed!(sint_i64_min, i64::MIN, &[0xd3u8, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // int64 min
}
