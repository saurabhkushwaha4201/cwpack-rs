pub mod ffi;
pub mod types;
pub mod pack;
pub mod unpack;

#[cfg(test)]
mod tests {
    use super::ffi::*;
    use std::ffi::c_void;

    // Helper: initialise a C pack context into a fixed buffer
    unsafe fn c_pack_init(buf: &mut [u8]) -> CwPackContext {
        let mut ctx = std::mem::MaybeUninit::<CwPackContext>::uninit();
        let rc = cw_pack_context_init(
            ctx.as_mut_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            buf.len() as std::os::raw::c_ulong,
            None,
        );
        assert_eq!(rc, CWP_RC_OK, "cw_pack_context_init failed");
        ctx.assume_init()
    }

    // Helper: initialise a C unpack context from a slice
    unsafe fn c_unpack_init(buf: &[u8]) -> CwUnpackContext {
        let mut ctx = std::mem::MaybeUninit::<CwUnpackContext>::uninit();
        let rc = cw_unpack_context_init(
            ctx.as_mut_ptr(),
            buf.as_ptr() as *const c_void,
            buf.len() as std::os::raw::c_ulong,
            None,
        );
        assert_eq!(rc, CWP_RC_OK, "cw_unpack_context_init failed");
        ctx.assume_init()
    }

    // -----------------------------------------------------------------------
    // Existing smoke tests (preserved from Phase 2)
    // -----------------------------------------------------------------------

    #[test]
    fn test_pack_unpack_str() {
        unsafe {
            let mut buffer: [u8; 1024] = [0; 1024];
            let mut pack_ctx = c_pack_init(&mut buffer);

            let msg = b"hello";
            cw_pack_str(&mut pack_ctx, msg.as_ptr() as *const i8, msg.len() as u32);

            let length = pack_ctx.current.offset_from(pack_ctx.start) as std::os::raw::c_ulong;

            let mut unpack_ctx = c_unpack_init(&buffer[..length as usize]);

            cw_unpack_next(&mut unpack_ctx);

            assert_eq!(unpack_ctx.item.type_, CWP_ITEM_STR, "Item should be a string");

            let unpacked_str_blob = unpack_ctx.item.as_.str_;
            assert_eq!(unpacked_str_blob.length, msg.len() as u32);

            let unpacked_slice = std::slice::from_raw_parts(
                unpacked_str_blob.start as *const u8,
                unpacked_str_blob.length as usize,
            );
            assert_eq!(unpacked_slice, msg);
        }
    }

    #[test]
    fn test_struct_layout_and_return_code() {
        unsafe {
            let mut buffer: [u8; 2] = [0; 2];
            let mut pack_ctx = c_pack_init(&mut buffer);

            assert_eq!(pack_ctx.return_code, 0);

            let msg = b"hello world";
            cw_pack_str(&mut pack_ctx, msg.as_ptr() as *const i8, msg.len() as u32);

            assert_eq!(
                pack_ctx.return_code, CWP_RC_BUFFER_OVERFLOW,
                "Struct layout misalignment: return_code was not updated correctly!"
            );
        }
    }

    // -----------------------------------------------------------------------
    // New: Rust-native API smoke tests
    // -----------------------------------------------------------------------

    #[test]
    fn rust_pack_nil_roundtrip() {
        use crate::pack::PackContext;
        use crate::unpack::UnpackContext;
        use crate::types::ITEM_NIL;

        let mut buf = [0u8; 64];
        let mut pctx = PackContext::new(&mut buf);
        pctx.pack_nil();
        assert_eq!(pctx.return_code, 0, "pack_nil should succeed");
        let n = pctx.written();

        let mut uctx = UnpackContext::new(&buf[..n]);
        uctx.unpack_next();
        assert_eq!(uctx.return_code, 0);
        assert_eq!(uctx.item.type_, ITEM_NIL);
    }

    #[test]
    fn rust_pack_unsigned_boundaries() {
        use crate::pack::PackContext;

        // fixint max → 1 byte
        let mut buf = [0u8; 64];
        let mut pctx = PackContext::new(&mut buf);
        pctx.pack_unsigned(127);
        assert_eq!(pctx.written(), 1);
        assert_eq!(buf[0], 0x7f);

        // uint8 min → 2 bytes
        let mut buf = [0u8; 64];
        let mut pctx = PackContext::new(&mut buf);
        pctx.pack_unsigned(128);
        assert_eq!(pctx.written(), 2);
        assert_eq!(&buf[..2], &[0xcc, 0x80]);

        // uint32 max → 5 bytes
        let mut buf = [0u8; 64];
        let mut pctx = PackContext::new(&mut buf);
        pctx.pack_unsigned(0xffff_ffff);
        assert_eq!(pctx.written(), 5);
        assert_eq!(&buf[..5], &[0xce, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn rust_pack_signed_fixint_for_0_to_127() {
        use crate::pack::PackContext;
        // REQ-FIX-002: signed 0–127 → fixint (NOT uint8 0xcc)
        for v in [0i64, 1, 64, 127] {
            let mut buf = [0u8; 64];
            let mut pctx = PackContext::new(&mut buf);
            pctx.pack_signed(v);
            assert_eq!(pctx.written(), 1, "signed({v}) should be fixint (1 byte)");
            assert_eq!(buf[0], v as u8, "signed({v}) fixint byte wrong");
        }
    }

    #[test]
    fn rust_timestamp32_bug001_not_reproduced() {
        use crate::pack::PackContext;
        use crate::unpack::UnpackContext;

        // BUG-001: sec=3_000_000_000 in [2^31, 2^32); C on Windows gives negative tv_sec
        // Rust must give positive tv_sec = 3_000_000_000
        let mut buf = [0u8; 64];
        let mut pctx = PackContext::new(&mut buf);
        pctx.pack_time(3_000_000_000i64, 0);
        assert_eq!(pctx.return_code, 0);

        let n = pctx.written();
        let mut uctx = UnpackContext::new(&buf[..n]);
        uctx.unpack_next();

        assert_eq!(uctx.return_code, 0);
        unsafe {
            assert_eq!(
                uctx.item.as_.time.tv_sec, 3_000_000_000i64,
                "BUG-001: tv_sec should be 3000000000, not negative"
            );
            assert_eq!(uctx.item.as_.time.tv_nsec, 0);
        }
    }

    #[test]
    fn rust_sticky_error_no_op() {
        use crate::pack::PackContext;
        use crate::types::RC_BUFFER_OVERFLOW;

        // Overflow buffer, then ensure subsequent calls are no-ops
        let mut buf = [0u8; 2];
        let mut pctx = PackContext::new(&mut buf);
        pctx.pack_str(b"hello world"); // causes overflow
        assert_eq!(pctx.return_code, RC_BUFFER_OVERFLOW);
        let pos_after_error = pctx.written();
        pctx.pack_nil(); // must be no-op
        pctx.pack_unsigned(42); // must be no-op
        assert_eq!(pctx.written(), pos_after_error, "No bytes should be written after error");
        assert_eq!(pctx.return_code, RC_BUFFER_OVERFLOW, "return_code must stay sticky");
    }
}
