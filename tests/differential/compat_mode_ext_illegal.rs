//! Differential tests for ext type in compat mode.
//!
//! Verifies that cw_pack_ext and cw_pack_time return ILLEGAL_CALL in compatibility mode.

#[cfg(test)]
mod compat_mode_ext_illegal {
    use cwpack_rs::ffi::{self, CwPackContext, CWP_RC_ILLEGAL_CALL};
    use cwpack_rs::pack::PackContext;
    use cwpack_rs::types::RC_ILLEGAL_CALL;
    use std::ffi::c_void;

    unsafe fn c_pack_ext_compat() -> i32 {
        let mut buf = [0u8; 64];
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(
            ctx.as_mut_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            64 as _,
            None,
        );
        let mut ctx = ctx.assume_init();
        ffi::cw_pack_set_compatibility(&mut ctx, true);
        
        let data = [0u8; 1];
        ffi::cw_pack_ext(&mut ctx, 1, data.as_ptr() as *const c_void, 1);
        ctx.return_code
    }

    unsafe fn c_pack_time_compat() -> i32 {
        let mut buf = [0u8; 64];
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(
            ctx.as_mut_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            64 as _,
            None,
        );
        let mut ctx = ctx.assume_init();
        ffi::cw_pack_set_compatibility(&mut ctx, true);
        
        ffi::cw_pack_time(&mut ctx, 0, 0);
        ctx.return_code
    }

    fn rust_pack_ext_compat() -> i32 {
        let mut buf = [0u8; 64];
        let mut ctx = PackContext::new(&mut buf);
        ctx.set_compatibility(true);
        let data = [0u8; 1];
        ctx.pack_ext(1, &data);
        ctx.return_code
    }

    fn rust_pack_time_compat() -> i32 {
        let mut buf = [0u8; 64];
        let mut ctx = PackContext::new(&mut buf);
        ctx.set_compatibility(true);
        ctx.pack_time(0, 0);
        ctx.return_code
    }

    #[test]
    fn test_compat_mode_ext_illegal() {
        let c_rc = unsafe { c_pack_ext_compat() };
        let rs_rc = rust_pack_ext_compat();
        assert_eq!(c_rc, CWP_RC_ILLEGAL_CALL);
        assert_eq!(rs_rc, RC_ILLEGAL_CALL);
    }

    #[test]
    fn test_compat_mode_time_illegal() {
        let c_rc = unsafe { c_pack_time_compat() };
        let rs_rc = rust_pack_time_compat();
        assert_eq!(c_rc, CWP_RC_ILLEGAL_CALL);
        assert_eq!(rs_rc, RC_ILLEGAL_CALL);
    }
}
