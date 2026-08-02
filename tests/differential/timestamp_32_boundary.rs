//! Differential tests for timestamp32 encoding/decoding.
//!
//! Compares output of C FFI (oracle) vs Rust port for timestamp32 boundary values.
//!
//! IMPORTANT NOTE on BUG-001:
//! The C oracle on Windows (LLP64) is KNOWN TO BE WRONG for sec >= 2^31.
//! Tests for those values explicitly verify the Rust port produces the CORRECT
//! spec-compliant result while documenting the C oracle's bug.

#[cfg(test)]
mod timestamp_32 {
    use cwpack_rs::ffi::{self, CwPackContext, CwUnpackContext, CWP_RC_OK};
    use cwpack_rs::pack::PackContext;
    use cwpack_rs::unpack::UnpackContext;
    use cwpack_rs::types::{ITEM_TIMESTAMP, RC_OK};
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
    // Pack tests: both C and Rust should produce identical bytes for all values
    // (timestamp32 encoding is byte-identical; BUG-001 is in UNPACK only)
    // -----------------------------------------------------------------------

    #[test]
    fn pack_timestamp32_sec_zero_nsec_zero() {
        let (c_bytes, c_rc) = unsafe { c_pack_time(0, 0) };
        let (rs_bytes, rs_rc) = rust_pack_time(0, 0);
        assert_eq!(c_rc, CWP_RC_OK);
        assert_eq!(rs_rc, RC_OK);
        assert_eq!(c_bytes, rs_bytes, "sec=0,nsec=0: pack bytes should match");
        // Verify the format: should be timestamp32 (6 bytes: 0xd6 0xff 0x00 0x00 0x00 0x00)
        assert_eq!(rs_bytes, &[0xd6, 0xff, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn pack_timestamp32_sec_1_nsec_0() {
        let (c_bytes, _) = unsafe { c_pack_time(1, 0) };
        let (rs_bytes, _) = rust_pack_time(1, 0);
        assert_eq!(c_bytes, rs_bytes, "sec=1,nsec=0");
        assert_eq!(rs_bytes, &[0xd6, 0xff, 0x00, 0x00, 0x00, 0x01]);
    }

    #[test]
    fn pack_timestamp32_max_u32_boundary() {
        // sec = 2^32 - 1 = 4294967295 — last value in timestamp32 range
        let (c_bytes, _) = unsafe { c_pack_time(4294967295, 0) };
        let (rs_bytes, _) = rust_pack_time(4294967295, 0);
        assert_eq!(c_bytes, rs_bytes, "sec=4294967295,nsec=0");
        assert_eq!(rs_bytes, &[0xd6, 0xff, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn pack_timestamp64_sec_0_nsec_nonzero() {
        // nsec > 0 forces timestamp64 even for small sec
        let (c_bytes, _) = unsafe { c_pack_time(0, 1) };
        let (rs_bytes, _) = rust_pack_time(0, 1);
        assert_eq!(c_bytes, rs_bytes, "sec=0,nsec=1 should be timestamp64");
        assert_eq!(rs_bytes.len(), 10, "timestamp64 is 10 bytes");
        assert_eq!(rs_bytes[0], 0xd7);
        assert_eq!(rs_bytes[1], 0xff);
    }

    // -----------------------------------------------------------------------
    // Unpack tests: BUG-001 section
    //
    // For sec < 2^31: C and Rust agree (both correct).
    // For sec in [2^31, 2^32): C is WRONG (gives negative), Rust is CORRECT.
    // -----------------------------------------------------------------------

    #[test]
    fn unpack_timestamp32_sec_below_2_31_agrees_with_c() {
        // sec = 2^31 - 1 = 2147483647 — last value where C is correct
        let (rs_bytes, _) = rust_pack_time(2147483647, 0);
        let (c_sec, c_nsec, c_rc) = unsafe { c_unpack_time(&rs_bytes) };
        let (rs_sec, rs_nsec, rs_rc) = rust_unpack_time(&rs_bytes);

        assert_eq!(c_rc, CWP_RC_OK);
        assert_eq!(rs_rc, RC_OK);
        // Both should agree for values < 2^31
        assert_eq!(c_sec, rs_sec, "C and Rust should agree for sec < 2^31");
        assert_eq!(c_nsec, rs_nsec);
        assert_eq!(rs_sec, 2147483647i64);
    }

    #[test]
    fn unpack_timestamp32_bug001_sec_at_2_31() {
        // sec = 2^31 = 2147483648 — first value where C is wrong on LLP64
        let (rs_bytes, _) = rust_pack_time(2147483648, 0);
        let (rs_sec, _rs_nsec, rs_rc) = rust_unpack_time(&rs_bytes);

        assert_eq!(rs_rc, RC_OK);
        // Rust must give the CORRECT positive value
        assert_eq!(
            rs_sec, 2147483648i64,
            "Rust port must correctly decode sec=2^31 as positive (BUG-001 fix)"
        );

        // C oracle on Windows WILL give -2147483648 (BUG-001)
        // We document this without asserting c_sec to avoid test failure on LP64 platforms
        let (c_sec, _c_nsec, c_rc) = unsafe { c_unpack_time(&rs_bytes) };
        assert_eq!(c_rc, CWP_RC_OK);
        // On Windows LLP64: c_sec == -2147483648i64 (the bug)
        // On Linux LP64:    c_sec == 2147483648i64 (correct by accident)
        if c_sec < 0 {
            // We're on LLP64 — bug confirmed
            eprintln!(
                "BUG-001 confirmed: C oracle gives sec={} (expected {})",
                c_sec, 2147483648i64
            );
        }
    }

    #[test]
    fn unpack_timestamp32_sec_3_billion() {
        // sec = 3,000,000,000 — solidly in the bug zone
        let (rs_bytes, _) = rust_pack_time(3_000_000_000i64, 0);
        let (rs_sec, rs_nsec, rs_rc) = rust_unpack_time(&rs_bytes);

        assert_eq!(rs_rc, RC_OK);
        assert_eq!(rs_sec, 3_000_000_000i64, "BUG-001: Rust must give correct positive value");
        assert_eq!(rs_nsec, 0);
    }
}
