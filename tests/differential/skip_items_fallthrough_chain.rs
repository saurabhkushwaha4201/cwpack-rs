//! Differential tests: cw_skip_items fallthrough chain (Tier 1 — flagged as high-risk).
//!
//! Tests the switch fallthrough chain in cw_skip_items for fixed-width items.
//! The C code uses C switch fallthrough to share skip logic across opcodes.
//! The Rust port must produce identical `current` positions after skipping.

#[cfg(test)]
mod skip_items_fallthrough {
    use cwpack_rs::ffi::{self, CwPackContext, CwUnpackContext, CWP_RC_OK};
    use cwpack_rs::unpack::UnpackContext;
    use cwpack_rs::types::{RC_OK, ITEM_POSITIVE_INTEGER};
    use std::ffi::c_void;

    // Pack a sentinel value 0x9999 after the item to skip, so we can verify
    // the cursor landed in the right place after skipping
    const SENTINEL: u64 = 0x9999;

    unsafe fn make_c_pack_ctx(buf: &mut [u8]) -> CwPackContext {
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(ctx.as_mut_ptr(), buf.as_mut_ptr() as *mut c_void, buf.len() as _, None);
        ctx.assume_init()
    }

    /// Build a message: [items_to_skip] ++ uint64(SENTINEL)
    /// Returns the packed bytes.
    unsafe fn build_msg_c(pack_fn: impl FnOnce(&mut CwPackContext)) -> Vec<u8> {
        let mut buf = [0u8; 512];
        let mut ctx = make_c_pack_ctx(&mut buf);
        pack_fn(&mut ctx);
        ffi::cw_pack_unsigned(&mut ctx, SENTINEL);
        assert_eq!(ctx.return_code, CWP_RC_OK);
        let n = ctx.current.offset_from(ctx.start) as usize;
        buf[..n].to_vec()
    }

    /// Skip 1 item in C, read next — should be SENTINEL.
    unsafe fn c_skip_and_read_sentinel(bytes: &[u8]) -> (u64, i32) {
        let mut ctx = std::mem::MaybeUninit::<CwUnpackContext>::uninit();
        ffi::cw_unpack_context_init(ctx.as_mut_ptr(), bytes.as_ptr() as *const c_void, bytes.len() as _, None);
        let mut ctx = ctx.assume_init();
        ffi::cw_skip_items(&mut ctx, 1);
        ffi::cw_unpack_next(&mut ctx);
        let rc = ctx.return_code;
        let v = ctx.item.as_.u64_;
        (v, rc)
    }

    /// Skip 1 item in Rust, read next — should be SENTINEL.
    fn rust_skip_and_read_sentinel(bytes: &[u8]) -> (u64, i32) {
        let mut ctx = UnpackContext::new(bytes);
        ctx.skip_items(1);
        ctx.unpack_next();
        let rc = ctx.return_code;
        let v = unsafe { ctx.item.as_.u64_ };
        (v, rc)
    }

    macro_rules! fallthrough_test {
        ($name:ident, $build:expr) => {
            #[test]
            fn $name() {
                let bytes = unsafe { build_msg_c($build) };
                let (c_val, c_rc) = unsafe { c_skip_and_read_sentinel(&bytes) };
                let (rs_val, rs_rc) = rust_skip_and_read_sentinel(&bytes);
                assert_eq!(c_rc, CWP_RC_OK, "C skip rc");
                assert_eq!(rs_rc, RC_OK, "Rust skip rc");
                assert_eq!(c_val, SENTINEL, "C sentinel mismatch");
                assert_eq!(rs_val, SENTINEL, "Rust sentinel mismatch");
                assert_eq!(c_val, rs_val, "C and Rust must agree on sentinel");
            }
        };
    }

    // Fixed-size skip cases (the fallthrough chain)
    fallthrough_test!(skip_uint8,  |ctx| unsafe { ffi::cw_pack_unsigned(ctx, 200) });         // 0xcc
    fallthrough_test!(skip_uint16, |ctx| unsafe { ffi::cw_pack_unsigned(ctx, 60000) });       // 0xcd
    fallthrough_test!(skip_uint32, |ctx| unsafe { ffi::cw_pack_unsigned(ctx, 0xdeadbeefu64) }); // 0xce
    fallthrough_test!(skip_uint64, |ctx| unsafe { ffi::cw_pack_unsigned(ctx, u64::MAX) });    // 0xcf
    fallthrough_test!(skip_int8,   |ctx| unsafe { ffi::cw_pack_signed(ctx, -33) });           // 0xd0
    fallthrough_test!(skip_int16,  |ctx| unsafe { ffi::cw_pack_signed(ctx, -129) });          // 0xd1
    fallthrough_test!(skip_int32,  |ctx| unsafe { ffi::cw_pack_signed(ctx, -32769) });        // 0xd2
    fallthrough_test!(skip_int64,  |ctx| unsafe { ffi::cw_pack_signed(ctx, i64::MIN) });      // 0xd3
    fallthrough_test!(skip_float,  |ctx| unsafe { ffi::cw_pack_float(ctx, 3.14f32) });           // 0xca
    fallthrough_test!(skip_double, |ctx| unsafe { ffi::cw_pack_double(ctx, 3.14f64) });          // 0xcb
    fallthrough_test!(skip_nil,    |ctx| unsafe { ffi::cw_pack_nil(ctx) });                   // 0xc0
    fallthrough_test!(skip_true,   |ctx| unsafe { ffi::cw_pack_true(ctx) });                  // 0xc3
    fallthrough_test!(skip_false,  |ctx| unsafe { ffi::cw_pack_false(ctx) });                 // 0xc2
    fallthrough_test!(skip_fixint_pos, |ctx| unsafe { ffi::cw_pack_unsigned(ctx, 64) });      // 0x40
    fallthrough_test!(skip_fixint_neg, |ctx| unsafe { ffi::cw_pack_signed(ctx, -1) });        // 0xff
    fallthrough_test!(skip_fixstr, |ctx| unsafe { ffi::cw_pack_str(ctx, b"hello\0".as_ptr() as *const i8, 5) }); // 0xa5
    fallthrough_test!(skip_str8,   |ctx| unsafe { ffi::cw_pack_str(ctx, [0u8; 32].as_ptr() as *const i8, 32u32) }); // 0xd9
    fallthrough_test!(skip_bin8,   |ctx| unsafe { ffi::cw_pack_bin(ctx, [0u8; 10].as_ptr() as *const c_void, 10) }); // 0xc4
    fallthrough_test!(skip_fixext1, |ctx| unsafe { ffi::cw_pack_ext(ctx, 5, [0u8;1].as_ptr() as *const c_void, 1) }); // 0xd4
    fallthrough_test!(skip_fixext2, |ctx| unsafe { ffi::cw_pack_ext(ctx, 5, [0u8;2].as_ptr() as *const c_void, 2) }); // 0xd5
    fallthrough_test!(skip_fixext4, |ctx| unsafe { ffi::cw_pack_ext(ctx, 5, [0u8;4].as_ptr() as *const c_void, 4) }); // 0xd6
    fallthrough_test!(skip_fixext8, |ctx| unsafe { ffi::cw_pack_ext(ctx, 5, [0u8;8].as_ptr() as *const c_void, 8) }); // 0xd7
    fallthrough_test!(skip_fixext16, |ctx| unsafe { ffi::cw_pack_ext(ctx, 5, [0u8;16].as_ptr() as *const c_void, 16) }); // 0xd8
    fallthrough_test!(skip_ext8,   |ctx| unsafe { ffi::cw_pack_ext(ctx, 5, [0u8;3].as_ptr() as *const c_void, 3) }); // 0xc7
    fallthrough_test!(skip_ext16,  |ctx| unsafe { ffi::cw_pack_ext(ctx, 5, [0u8;256].as_ptr() as *const c_void, 256u32) }); // 0xc8
}
