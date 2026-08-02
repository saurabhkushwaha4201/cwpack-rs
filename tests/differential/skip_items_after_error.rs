//! Differential tests for cw_skip_items after an error.
//!
//! Verifies that cw_skip_items does nothing if the context has an error.

#[cfg(test)]
mod skip_items_after_error {
    use cwpack_rs::ffi::{self, CwUnpackContext, CWP_RC_END_OF_INPUT};
    use cwpack_rs::unpack::UnpackContext;
    use std::ffi::c_void;

    unsafe fn c_skip_error() -> usize {
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
        
        let ptr_before = ctx.current;
        ffi::cw_skip_items(&mut ctx, 1);
        let ptr_after = ctx.current;
        
        ptr_after.offset_from(ptr_before) as usize
    }

    fn rust_skip_error() -> usize {
        let bytes = [];
        let mut ctx = UnpackContext::new(&bytes);
        ctx.unpack_next();
        
        let pos_before = ctx.current_pos();
        ctx.skip_items(1);
        let pos_after = ctx.current_pos();
        
        pos_after - pos_before
    }

    #[test]
    fn test_skip_after_error() {
        let c_res = unsafe { c_skip_error() };
        let rs_res = rust_skip_error();
        
        assert_eq!(c_res, 0);
        assert_eq!(rs_res, 0);
    }
}
