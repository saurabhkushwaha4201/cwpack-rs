//! Differential tests for compatibility mode bin-to-str encoding.
//!
//! Verifies that when be_compatible is true, cw_pack_bin behaves like cw_pack_str.

#[cfg(test)]
mod compat_mode_bin_to_str {
    use cwpack_rs::ffi::{self, CwPackContext, CWP_RC_OK};
    use cwpack_rs::pack::PackContext;
    use cwpack_rs::types::RC_OK;
    use std::ffi::c_void;

    unsafe fn c_pack_bin_compat(data: &[u8]) -> (Vec<u8>, i32) {
        let mut buf = [0u8; 1024];
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        ffi::cw_pack_context_init(
            ctx.as_mut_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            buf.len() as _,
            None,
        );
        let mut ctx = ctx.assume_init();
        ffi::cw_pack_set_compatibility(&mut ctx, true);
        ffi::cw_pack_bin(&mut ctx, data.as_ptr() as *const c_void, data.len() as u32);
        
        let rc = ctx.return_code;
        let len = ctx.current.offset_from(ctx.start) as usize;
        (buf[..len].to_vec(), rc)
    }

    fn rust_pack_bin_compat(data: &[u8]) -> (Vec<u8>, i32) {
        let mut buf = [0u8; 1024];
        let mut ctx = PackContext::new(&mut buf);
        ctx.set_compatibility(true);
        ctx.pack_bin(data);
        
        let rc = ctx.return_code;
        let len = ctx.written();
        (buf[..len].to_vec(), rc)
    }

    macro_rules! compat_bin_test {
        ($name:ident, $len:expr) => {
            #[test]
            fn $name() {
                let data = vec![0x42u8; $len];
                let (c_bytes, c_rc) = unsafe { c_pack_bin_compat(&data) };
                let (rs_bytes, rs_rc) = rust_pack_bin_compat(&data);

                assert_eq!(c_rc, CWP_RC_OK);
                assert_eq!(rs_rc, RC_OK);
                assert_eq!(c_bytes, rs_bytes, "compat mode bin encoding mismatch for length {}", $len);
                
                // For compat mode, bin becomes str.
                // length < 32 -> fixstr (0xa0)
                // length < 256 -> str16 (0xda) (str8 is skipped in compat mode)
                // length < 65536 -> str16 (0xda)
                if $len < 32 {
                    let len_u8 = ($len & 0xff) as u8;
                    assert_eq!(rs_bytes[0], 0xa0 | len_u8);
                } else if $len < 65536 {
                    assert_eq!(rs_bytes[0], 0xda);
                } else {
                    assert_eq!(rs_bytes[0], 0xdb);
                }
            }
        };
    }

    compat_bin_test!(test_bin_compat_fixstr, 10);
    compat_bin_test!(test_bin_compat_str16_skip_str8, 100);
    compat_bin_test!(test_bin_compat_str16, 300);
}
