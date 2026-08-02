//! Differential tests for str8 skipping in compat mode.
//!
//! Verifies that cw_pack_str with length 32..255 skips str8 (0xd9) and uses str16 (0xda)
//! in compatibility mode.

#[cfg(test)]
mod compat_mode_str8_skip {
    use cwpack_rs::ffi::{self, CwPackContext, CWP_RC_OK};
    use cwpack_rs::pack::PackContext;
    use cwpack_rs::types::RC_OK;
    use std::ffi::c_void;

    unsafe fn c_pack_str_compat(data: &[u8]) -> (Vec<u8>, i32) {
        let mut buf = [0u8; 512];
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(
            ctx.as_mut_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            buf.len() as _,
            None,
        );
        let mut ctx = ctx.assume_init();
        ffi::cw_pack_set_compatibility(&mut ctx, true);
        ffi::cw_pack_str(&mut ctx, data.as_ptr() as *const i8, data.len() as u32);
        
        let rc = ctx.return_code;
        let len = ctx.current.offset_from(ctx.start) as usize;
        (buf[..len].to_vec(), rc)
    }

    fn rust_pack_str_compat(data: &[u8]) -> (Vec<u8>, i32) {
        let mut buf = [0u8; 512];
        let mut ctx = PackContext::new(&mut buf);
        ctx.set_compatibility(true);
        ctx.pack_str(data);
        
        let rc = ctx.return_code;
        let len = ctx.written();
        (buf[..len].to_vec(), rc)
    }

    #[test]
    fn test_str8_skipped_in_compat() {
        let data = vec![0x41u8; 100]; // Length 100 would normally be str8
        
        let (c_bytes, c_rc) = unsafe { c_pack_str_compat(&data) };
        let (rs_bytes, rs_rc) = rust_pack_str_compat(&data);

        assert_eq!(c_rc, CWP_RC_OK);
        assert_eq!(rs_rc, RC_OK);
        assert_eq!(c_bytes, rs_bytes, "compat mode str encoding mismatch");
        
        // str16 tag is 0xda
        assert_eq!(rs_bytes[0], 0xda, "expected str16 (0xda), got {:#x}", rs_bytes[0]);
    }
}
