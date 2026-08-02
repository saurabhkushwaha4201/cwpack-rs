//! Differential tests for timestamp invalid nsec handling.
//!
//! Verifies that cw_pack_time with nsec >= 1_000_000_000 returns VALUE_ERROR.

#[cfg(test)]
mod timestamp_invalid_nsec {
    use cwpack_rs::ffi::{self, CwPackContext, CWP_RC_VALUE_ERROR};
    use cwpack_rs::pack::PackContext;
    use cwpack_rs::types::RC_VALUE_ERROR;
    use std::ffi::c_void;

    unsafe fn c_pack_time(sec: i64, nsec: u32) -> i32 {
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
        ctx.return_code
    }

    fn rust_pack_time(sec: i64, nsec: u32) -> i32 {
        let mut buf = [0u8; 64];
        let mut ctx = PackContext::new(&mut buf);
        ctx.pack_time(sec, nsec);
        ctx.return_code
    }

    #[test]
    fn test_invalid_nsec() {
        let sec = 0;
        let nsec = 1_000_000_000;
        let c_rc = unsafe { c_pack_time(sec, nsec) };
        let rs_rc = rust_pack_time(sec, nsec);

        assert_eq!(c_rc, CWP_RC_VALUE_ERROR);
        assert_eq!(rs_rc, RC_VALUE_ERROR);
    }
}
