//! Differential tests for timestamp64 encoding/decoding.
//!
//! Compares output of C FFI (oracle) vs Rust port for timestamp64 boundary values.

#[cfg(test)]
mod timestamp_64 {
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

    macro_rules! timestamp64_test {
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
                // Verify length is 10 for timestamp64
                assert_eq!(rs_bytes.len(), 10, "timestamp64 must be 10 bytes");
                assert_eq!(rs_bytes[0], 0xd7, "timestamp64 starts with 0xd7");

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

    // 1. sec = 0, nsec = 1 (lowest boundary forcing timestamp64)
    timestamp64_test!(ts64_sec_0_nsec_1, 0, 1);

    // 2. sec = 2^32 - 1, nsec = 1
    timestamp64_test!(ts64_sec_max32_nsec_1, 4294967295, 1);

    // 3. sec = 2^32, nsec = 0 (lowest sec forcing timestamp64 when nsec=0)
    timestamp64_test!(ts64_sec_2_32_nsec_0, 4294967296, 0);

    // 4. sec = 2^34 - 1, nsec = 0 (highest boundary for timestamp64)
    timestamp64_test!(ts64_sec_max34_nsec_0, 17179869183, 0);
    
    // 5. sec = 2^34 - 1, nsec = 999_999_999 (absolute max valid timestamp64)
    timestamp64_test!(ts64_sec_max34_nsec_max, 17179869183, 999_999_999);
}
