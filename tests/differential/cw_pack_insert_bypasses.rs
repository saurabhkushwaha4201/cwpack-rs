//! Differential tests for cw_pack_insert sticky error bypass.
//!
//! Verifies that cw_pack_insert ignores sticky errors and writes anyway.

#[cfg(test)]
mod pack_insert_bypasses {
    use cwpack_rs::ffi::{self, CwPackContext, CWP_RC_BUFFER_OVERFLOW};
    use cwpack_rs::pack::PackContext;
    use cwpack_rs::types::RC_BUFFER_OVERFLOW;
    use std::ffi::c_void;

    unsafe fn c_pack_insert_bypass() -> (usize, i32) {
        let mut buf = [0u8; 10];
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(
            ctx.as_mut_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            2 as _,
            None,
        );
        let mut ctx = ctx.assume_init();
        
        // This will fail (buffer overflow)
        ffi::cw_pack_unsigned(&mut ctx, 0x12345678);
        assert_eq!(ctx.return_code, CWP_RC_BUFFER_OVERFLOW);

        let data = [0x42u8, 0x43u8];
        // But insert ignores the error! It will write if there's physical space,
        // wait, does it check physical space? `cw_pack_reserve_space` does check space!
        // But because the context has a tiny limit (2 bytes), and insert asks for 2 bytes,
        // it WILL succeed to insert! 
        
        // Let's reset the buffer bounds to be larger so insert can work.
        // Or wait, if we init with length 10, then fill it, then overflow...
        // Let's just init with length 10.
        ffi::cw_pack_context_init(
            &mut ctx as *mut _ as *mut CwPackContext,
            buf.as_mut_ptr() as *mut c_void,
            10 as _,
            None,
        );
        
        // Set an error manually
        ctx.return_code = CWP_RC_BUFFER_OVERFLOW;
        
        // Now insert
        ffi::cw_pack_insert(&mut ctx, data.as_ptr() as *const c_void, 2);
        
        let len = ctx.current.offset_from(ctx.start) as usize;
        (len, ctx.return_code)
    }

    fn rust_pack_insert_bypass() -> (usize, i32) {
        let mut buf = [0u8; 10];
        let mut ctx = PackContext::new(&mut buf);
        
        ctx.return_code = RC_BUFFER_OVERFLOW;
        
        let data = [0x42u8, 0x43u8];
        ctx.pack_insert(&data);
        
        (ctx.written(), ctx.return_code)
    }

    #[test]
    fn test_insert_bypasses_error() {
        let (c_len, c_rc) = unsafe { c_pack_insert_bypass() };
        let (rs_len, rs_rc) = rust_pack_insert_bypass();
        
        // Both should write 2 bytes, despite the error.
        assert_eq!(c_len, 2);
        assert_eq!(rs_len, 2);
        
        assert_eq!(c_rc, CWP_RC_BUFFER_OVERFLOW);
        assert_eq!(rs_rc, RC_BUFFER_OVERFLOW);
    }
}
