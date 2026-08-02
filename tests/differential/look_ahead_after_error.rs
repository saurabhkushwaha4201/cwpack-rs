//! Differential tests for cw_look_ahead after an error.
//!
//! Verifies that look_ahead returns CWP_NOT_AN_ITEM if context has an error.

#[cfg(test)]
mod look_ahead_after_error {
    use cwpack_rs::ffi::{self, CwUnpackContext, CWP_NOT_AN_ITEM, CWP_RC_END_OF_INPUT};
    use cwpack_rs::unpack::UnpackContext;
    use cwpack_rs::types::ITEM_NOT_AN_ITEM;
    use std::ffi::c_void;

    unsafe fn c_look_ahead_error() -> i32 {
        let mut ctx = std::mem::MaybeUninit::<CwUnpackContext>::uninit();
        let bytes = [];
        ffi::cw_unpack_context_init(
            ctx.as_mut_ptr(),
            bytes.as_ptr() as *const c_void,
            0,
            None,
        );
        let mut ctx = ctx.assume_init();
        
        // This will set END_OF_INPUT
        ffi::cw_unpack_next(&mut ctx);
        assert_eq!(ctx.return_code, CWP_RC_END_OF_INPUT);
        
        ffi::cw_look_ahead(&mut ctx)
    }

    fn rust_look_ahead_error() -> i32 {
        let bytes = [];
        let mut ctx = UnpackContext::new(&bytes);
        ctx.unpack_next();
        ctx.look_ahead()
    }

    #[test]
    fn test_look_ahead_after_error() {
        let c_res = unsafe { c_look_ahead_error() };
        let rs_res = rust_look_ahead_error();
        
        assert_eq!(c_res, CWP_NOT_AN_ITEM);
        assert_eq!(rs_res, ITEM_NOT_AN_ITEM);
    }
}
