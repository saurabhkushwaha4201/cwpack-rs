//! Differential tests for timestamp96 encoding/decoding.
//!
//! Compares output of C FFI (oracle) vs Rust port for timestamp96 boundary values.

#[cfg(test)]
mod timestamp_96 {
    use cwpack_rs::ffi::{self, CwPackContext, CwUnpackContext, CWP_RC_OK};
    use cwpack_rs::pack::PackContext;
    use cwpack_rs::unpack::UnpackContext;
    use cwpack_rs::types::RC_OK;
    use std::ffi::c_void;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    unsafe fn c_pack_time(sec: i64, nsec: u32) -> (Vec<u8>, i32) {
        let mut buf = [0u8; 64];
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(
            ctx.as_mut_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            64 as _,
            None,
        );
        let mut ctx = ctx.assume_init();
        ffi::cw_pack_time(&mut ctx, sec, nsec);
        let rc = ctx.return_code;
        if rc != 0 {
            return (vec![], rc);
        }
        let len = ctx.current.offset_from(ctx.start) as usize;
        (buf[..len].to_vec(), rc)
    }

    fn rust_pack_time(sec: i64, nsec: u32) -> (Vec<u8>, i32) {
        let mut buf = [0u8; 64];
        let mut ctx = PackContext::new(&mut buf);
        ctx.pack_time(sec, nsec);
        let rc = ctx.return_code;
        let n = ctx.written();
        (buf[..n].to_vec(), rc)
    }

    unsafe fn c_unpack_time(bytes: &[u8]) -> (i64, u32, i32) {
        let mut ctx = std::mem::MaybeUninit::<CwUnpackContext>::uninit();
        ffi::cw_unpack_context_init(
            ctx.as_mut_ptr(),
            bytes.as_ptr() as *const c_void,
            bytes.len() as _,
            None,
        );
        let mut ctx = ctx.assume_init();
        ffi::cw_unpack_next(&mut ctx);
        let rc = ctx.return_code;
        let sec = ctx.item.as_.time.tv_sec;
        let nsec = ctx.item.as_.time.tv_nsec;
        (sec, nsec, rc)
    }

    fn rust_unpack_time(bytes: &[u8]) -> (i64, u32, i32) {
        let mut ctx = UnpackContext::new(bytes);
        ctx.unpack_next();
        let rc = ctx.return_code;
        let (sec, nsec) = unsafe { (ctx.item.as_.time.tv_sec, ctx.item.as_.time.tv_nsec) };
        (sec, nsec, rc)
    }

    // -----------------------------------------------------------------------
    // Pack & Unpack Tests
    // -----------------------------------------------------------------------

    macro_rules! timestamp96_test {
        ($name:ident, $sec:expr, $nsec:expr) => {
            #[test]
            fn $name() {
                let sec = $sec;
                let nsec = $nsec;
                // Pack
                let (c_bytes, c_pack_rc) = unsafe { c_pack_time(sec, nsec) };
                let (rs_bytes, rs_pack_rc) = rust_pack_time(sec, nsec);
                assert_eq!(c_pack_rc, CWP_RC_OK);
                assert_eq!(rs_pack_rc, RC_OK);
                assert_eq!(c_bytes, rs_bytes, "pack bytes mismatch for sec={}, nsec={}", sec, nsec);
                // Verify length is 15 for timestamp96
                assert_eq!(rs_bytes.len(), 15, "timestamp96 must be 15 bytes");
                assert_eq!(rs_bytes[0], 0xc7, "timestamp96 starts with ext8 (0xc7)");

                // Unpack
                let (c_sec, c_nsec, c_unpack_rc) = unsafe { c_unpack_time(&rs_bytes) };
                let (rs_sec, rs_nsec, rs_unpack_rc) = rust_unpack_time(&rs_bytes);
                assert_eq!(c_unpack_rc, CWP_RC_OK);
                assert_eq!(rs_unpack_rc, RC_OK);
                assert_eq!(c_sec, rs_sec, "unpack sec mismatch");
                assert_eq!(c_nsec, rs_nsec, "unpack nsec mismatch");
                assert_eq!(rs_sec, sec, "unpacked sec does not match original");
                assert_eq!(rs_nsec, nsec, "unpacked nsec does not match original");
            }
        };
    }

    // 1. sec = -1, nsec = 0 (first negative value forcing timestamp96)
    timestamp96_test!(ts96_sec_neg1_nsec_0, -1, 0);

    // 2. sec = INT64_MIN, nsec = 0
    timestamp96_test!(ts96_sec_min_nsec_0, i64::MIN, 0);

    // 3. sec = 2^34, nsec = 0 (first positive value forcing timestamp96)
    timestamp96_test!(ts96_sec_2_34_nsec_0, 17179869184, 0);

    // 4. sec = INT64_MAX, nsec = 999_999_999
    timestamp96_test!(ts96_sec_max_nsec_max, i64::MAX, 999_999_999);
}
