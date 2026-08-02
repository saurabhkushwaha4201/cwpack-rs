//! Differential tests for invalid timestamp length unpacking.
//!
//! Verifies that fixext1, fixext2, and fixext16 with type == -1
//! return WRONG_TIMESTAMP_LENGTH during unpacking.

#[cfg(test)]
mod timestamp_fixext_wrong_len {
    use cwpack_rs::ffi::{self, CwUnpackContext, CWP_RC_WRONG_TIMESTAMP_LENGTH};
    use cwpack_rs::unpack::UnpackContext;
    use cwpack_rs::types::RC_WRONG_TIMESTAMP_LENGTH;
    use std::ffi::c_void;

    unsafe fn c_unpack_invalid_timestamp(bytes: &[u8]) -> i32 {
        let mut ctx = std::mem::MaybeUninit::<CwUnpackContext>::uninit();
        ffi::cw_unpack_context_init(
            ctx.as_mut_ptr(),
            bytes.as_ptr() as *const c_void,
            bytes.len() as _,
            None,
        );
        let mut ctx = ctx.assume_init();
        ffi::cw_unpack_next(&mut ctx);
        ctx.return_code
    }

    fn rust_unpack_invalid_timestamp(bytes: &[u8]) -> i32 {
        let mut ctx = UnpackContext::new(bytes);
        ctx.unpack_next();
        ctx.return_code
    }

    #[test]
    fn test_fixext1_timestamp() {
        // fixext1 (0xd4), type -1 (0xff), data 1 byte (0x00)
        let bytes = [0xd4, 0xff, 0x00];
        let c_rc = unsafe { c_unpack_invalid_timestamp(&bytes) };
        let rs_rc = rust_unpack_invalid_timestamp(&bytes);

        assert_eq!(c_rc, CWP_RC_WRONG_TIMESTAMP_LENGTH);
        assert_eq!(rs_rc, RC_WRONG_TIMESTAMP_LENGTH);
    }

    #[test]
    fn test_fixext2_timestamp() {
        // fixext2 (0xd5), type -1 (0xff), data 2 bytes
        let bytes = [0xd5, 0xff, 0x00, 0x00];
        let c_rc = unsafe { c_unpack_invalid_timestamp(&bytes) };
        let rs_rc = rust_unpack_invalid_timestamp(&bytes);

        assert_eq!(c_rc, CWP_RC_WRONG_TIMESTAMP_LENGTH);
        assert_eq!(rs_rc, RC_WRONG_TIMESTAMP_LENGTH);
    }

    #[test]
    fn test_fixext16_timestamp() {
        // fixext16 (0xd8), type -1 (0xff), data 16 bytes
        let mut bytes = vec![0xd8, 0xff];
        bytes.extend_from_slice(&[0x00; 16]);
        let c_rc = unsafe { c_unpack_invalid_timestamp(&bytes) };
        let rs_rc = rust_unpack_invalid_timestamp(&bytes);

        assert_eq!(c_rc, CWP_RC_WRONG_TIMESTAMP_LENGTH);
        assert_eq!(rs_rc, RC_WRONG_TIMESTAMP_LENGTH);
    }
}
