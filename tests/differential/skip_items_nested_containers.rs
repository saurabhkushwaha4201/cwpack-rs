//! Differential tests: cw_skip_items nested container arithmetic.
//!
//! Tests that skipping arrays and maps correctly accumulates child items
//! and that the cursor ends up in the right place to read the sentinel.

#[cfg(test)]
mod skip_nested_containers {
    use cwpack_rs::ffi::{self, CwPackContext, CwUnpackContext, CWP_RC_OK};
    use cwpack_rs::unpack::UnpackContext;
    use cwpack_rs::types::RC_OK;
    use std::ffi::c_void;

    const SENTINEL: u64 = 0xbeefdead;

    unsafe fn make_ctx(buf: &mut [u8]) -> CwPackContext {
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(ctx.as_mut_ptr(), buf.as_mut_ptr() as *mut c_void, buf.len() as _, None);
        ctx.assume_init()
    }

    unsafe fn c_skip_read(bytes: &[u8]) -> (u64, i32) {
        let mut ctx = std::mem::MaybeUninit::<CwUnpackContext>::uninit();
        ffi::cw_unpack_context_init(ctx.as_mut_ptr(), bytes.as_ptr() as *const c_void, bytes.len() as _, None);
        let mut ctx = ctx.assume_init();
        ffi::cw_skip_items(&mut ctx, 1);
        ffi::cw_unpack_next(&mut ctx);
        (ctx.item.as_.u64_, ctx.return_code)
    }

    fn rust_skip_read(bytes: &[u8]) -> (u64, i32) {
        let mut ctx = UnpackContext::new(bytes);
        ctx.skip_items(1);
        ctx.unpack_next();
        let rc = ctx.return_code;
        let v = unsafe { ctx.item.as_.u64_ };
        (v, rc)
    }

    macro_rules! container_skip_test {
        ($name:ident, $build:expr) => {
            #[test]
            fn $name() {
                let mut buf = [0u8; 1024];
                let bytes = unsafe {
                    let mut ctx = make_ctx(&mut buf);
                    $build(&mut ctx);
                    ffi::cw_pack_unsigned(&mut ctx, SENTINEL);
                    assert_eq!(ctx.return_code, CWP_RC_OK);
                    let n = ctx.current.offset_from(ctx.start) as usize;
                    buf[..n].to_vec()
                };
                let (c_val, c_rc) = unsafe { c_skip_read(&bytes) };
                let (rs_val, rs_rc) = rust_skip_read(&bytes);
                assert_eq!(c_rc, CWP_RC_OK);
                assert_eq!(rs_rc, RC_OK);
                assert_eq!(c_val, SENTINEL);
                assert_eq!(rs_val, SENTINEL);
                assert_eq!(c_val, rs_val, "C and Rust sentinel must agree");
            }
        };
    }

    // fixarray of 3 unsigned ints
    container_skip_test!(skip_fixarray_3, |ctx| {
        ffi::cw_pack_array_size(ctx, 3u32);
        ffi::cw_pack_unsigned(ctx, 1u64);
        ffi::cw_pack_unsigned(ctx, 2u64);
        ffi::cw_pack_unsigned(ctx, 3u64);
    });

    // fixmap of 2 key-value pairs
    container_skip_test!(skip_fixmap_2, |ctx| {
        ffi::cw_pack_map_size(ctx, 2u32);
        ffi::cw_pack_str(ctx, b"k1\0".as_ptr() as *const i8, 2u32);
        ffi::cw_pack_unsigned(ctx, 100u64);
        ffi::cw_pack_str(ctx, b"k2\0".as_ptr() as *const i8, 2u32);
        ffi::cw_pack_unsigned(ctx, 200u64);
    });

    // nested: array containing an array
    container_skip_test!(skip_nested_arrays, |ctx| {
        ffi::cw_pack_array_size(ctx, 2u32);
        // inner array of 2
        ffi::cw_pack_array_size(ctx, 2u32);
        ffi::cw_pack_unsigned(ctx, 10u64);
        ffi::cw_pack_unsigned(ctx, 20u64);
        // second element
        ffi::cw_pack_unsigned(ctx, 30u64);
    });

    // array16 (n=16)
    container_skip_test!(skip_array16, |ctx| {
        ffi::cw_pack_array_size(ctx, 16);
        for i in 0u64..16 {
            ffi::cw_pack_unsigned(ctx, i);
        }
    });

    // map16 (n=2 pairs = 4 items)
    container_skip_test!(skip_map16, |ctx| {
        ffi::cw_pack_map_size(ctx, 16);
        for i in 0u64..16 {
            ffi::cw_pack_unsigned(ctx, i);       // key
            ffi::cw_pack_unsigned(ctx, i * 10);  // value
        }
    });

    // Skip 3 items at once (skip_items count=3)
    #[test]
    fn skip_3_items() {
        let mut buf = [0u8; 128];
        let bytes = unsafe {
            let mut ctx = make_ctx(&mut buf);
            ffi::cw_pack_unsigned(&mut ctx, 1);
            ffi::cw_pack_unsigned(&mut ctx, 2);
            ffi::cw_pack_unsigned(&mut ctx, 3);
            ffi::cw_pack_unsigned(&mut ctx, SENTINEL);
            let n = ctx.current.offset_from(ctx.start) as usize;
            buf[..n].to_vec()
        };
        // C: skip 3 items
        let (c_val, c_rc) = unsafe {
            let mut ctx = std::mem::MaybeUninit::<CwUnpackContext>::uninit();
            ffi::cw_unpack_context_init(ctx.as_mut_ptr(), bytes.as_ptr() as *const c_void, bytes.len() as _, None);
            let mut ctx = ctx.assume_init();
            ffi::cw_skip_items(&mut ctx, 3);
            ffi::cw_unpack_next(&mut ctx);
            (ctx.item.as_.u64_, ctx.return_code)
        };
        // Rust: skip 3 items
        let (rs_val, rs_rc) = {
            let mut ctx = UnpackContext::new(&bytes);
            ctx.skip_items(3);
            ctx.unpack_next();
            let rc = ctx.return_code;
            let v = unsafe { ctx.item.as_.u64_ };
            (v, rc)
        };
        assert_eq!(c_rc, CWP_RC_OK);
        assert_eq!(rs_rc, RC_OK);
        assert_eq!(c_val, SENTINEL);
        assert_eq!(rs_val, SENTINEL);
    }
}
