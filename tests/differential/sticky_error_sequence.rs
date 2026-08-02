//! Differential tests for sticky error sequence handling.
//!
//! Verifies that after an error occurs, subsequent calls are no-ops
//! and the error code is retained.

#[cfg(test)]
mod sticky_error_sequence {
    use cwpack_rs::ffi::{self, CwPackContext, CWP_RC_OK, CWP_RC_BUFFER_OVERFLOW};
    use cwpack_rs::pack::PackContext;
    use cwpack_rs::types::RC_BUFFER_OVERFLOW;
    use std::ffi::c_void;

    unsafe fn c_pack_sequence() -> (i32, usize) {
        let mut buf = [0u8; 2];
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(
            ctx.as_mut_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            2 as _,
            None,
        );
        let mut ctx = ctx.assume_init();
        
        // This will succeed and fill the buffer (2 bytes)
        ffi::cw_pack_unsigned(&mut ctx, 128); // takes 2 bytes
        assert_eq!(ctx.return_code, CWP_RC_OK);

        // This will fail (buffer overflow)
        ffi::cw_pack_nil(&mut ctx);
        assert_eq!(ctx.return_code, CWP_RC_BUFFER_OVERFLOW);

        let written_before = ctx.current.offset_from(ctx.start) as usize;

        // This should be a no-op
        ffi::cw_pack_unsigned(&mut ctx, 42);

        let written_after = ctx.current.offset_from(ctx.start) as usize;

        assert_eq!(written_before, written_after);

        (ctx.return_code, written_after)
    }

    fn rust_pack_sequence() -> (i32, usize) {
        let mut buf = [0u8; 2];
        let mut ctx = PackContext::new(&mut buf);
        
        ctx.pack_unsigned(128);
        assert_eq!(ctx.return_code, 0);

        ctx.pack_nil();
        assert_eq!(ctx.return_code, RC_BUFFER_OVERFLOW);

        let written_before = ctx.written();

        ctx.pack_unsigned(42);

        let written_after = ctx.written();

        assert_eq!(written_before, written_after);

        (ctx.return_code, written_after)
    }

    #[test]
    fn test_sticky_error() {
        let (c_rc, c_len) = unsafe { c_pack_sequence() };
        let (rs_rc, rs_len) = rust_pack_sequence();

        assert_eq!(c_rc, CWP_RC_BUFFER_OVERFLOW);
        assert_eq!(rs_rc, RC_BUFFER_OVERFLOW);
        assert_eq!(c_len, rs_len);
    }
}
