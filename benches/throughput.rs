use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cwpack_rs::pack::PackContext;
use cwpack_rs::unpack::UnpackContext;
use cwpack_rs::ffi;
use std::ffi::c_void;

fn bench_pack_cwpack_rs(c: &mut Criterion) {
    let mut buf = [0u8; 1024];
    c.bench_function("pack_cwpack_rs", |b| b.iter(|| {
        let mut ctx = PackContext::new(&mut buf);
        ctx.pack_array_size(3);
        ctx.pack_unsigned(black_box(42));
        ctx.pack_str(black_box(b"hello world"));
        ctx.pack_map_size(1);
        ctx.pack_str(black_box(b"key"));
        ctx.pack_boolean(black_box(true));
    }));
}

fn bench_pack_cwpack_c(c: &mut Criterion) {
    let mut buf = [0u8; 1024];
    c.bench_function("pack_cwpack_c", |b| b.iter(|| {
        unsafe {
            let mut ctx = std::mem::MaybeUninit::<ffi::CwPackContext>::uninit();
            ffi::cw_pack_context_init(ctx.as_mut_ptr(), buf.as_mut_ptr() as *mut c_void, buf.len() as _, None);
            let mut ctx = ctx.assume_init();
            ffi::cw_pack_array_size(&mut ctx, 3);
            ffi::cw_pack_unsigned(&mut ctx, black_box(42));
            ffi::cw_pack_str(&mut ctx, black_box(b"hello world\0".as_ptr() as *const i8), 11);
            ffi::cw_pack_map_size(&mut ctx, 1);
            ffi::cw_pack_str(&mut ctx, black_box(b"key\0".as_ptr() as *const i8), 3);
            ffi::cw_pack_boolean(&mut ctx, black_box(true));
        }
    }));
}

fn bench_pack_rmp(c: &mut Criterion) {
    let mut buf = Vec::with_capacity(1024);
    c.bench_function("pack_rmp", |b| b.iter(|| {
        buf.clear();
        rmp::encode::write_array_len(&mut buf, 3).unwrap();
        rmp::encode::write_uint(&mut buf, black_box(42)).unwrap();
        rmp::encode::write_str(&mut buf, black_box("hello world")).unwrap();
        rmp::encode::write_map_len(&mut buf, 1).unwrap();
        rmp::encode::write_str(&mut buf, black_box("key")).unwrap();
        rmp::encode::write_bool(&mut buf, black_box(true)).unwrap();
    }));
}

fn bench_unpack_cwpack_rs(c: &mut Criterion) {
    let mut buf = [0u8; 1024];
    let mut pack_ctx = PackContext::new(&mut buf);
    pack_ctx.pack_array_size(3);
    pack_ctx.pack_unsigned(42);
    pack_ctx.pack_str(b"hello world");
    pack_ctx.pack_map_size(1);
    pack_ctx.pack_str(b"key");
    pack_ctx.pack_boolean(true);
    let bytes = pack_ctx.buffer_slice();

    c.bench_function("unpack_cwpack_rs", |b| b.iter(|| {
        let mut ctx = UnpackContext::new(bytes);
        ctx.unpack_next();
        ctx.unpack_next();
        ctx.unpack_next();
        ctx.unpack_next();
        ctx.unpack_next();
        ctx.unpack_next();
        black_box(ctx.return_code);
    }));
}

fn bench_unpack_cwpack_c(c: &mut Criterion) {
    let mut buf = [0u8; 1024];
    let mut pack_ctx = PackContext::new(&mut buf);
    pack_ctx.pack_array_size(3);
    pack_ctx.pack_unsigned(42);
    pack_ctx.pack_str(b"hello world");
    pack_ctx.pack_map_size(1);
    pack_ctx.pack_str(b"key");
    pack_ctx.pack_boolean(true);
    let bytes = pack_ctx.buffer_slice();

    c.bench_function("unpack_cwpack_c", |b| b.iter(|| {
        unsafe {
            let mut ctx = std::mem::MaybeUninit::<ffi::CwUnpackContext>::uninit();
            ffi::cw_unpack_context_init(ctx.as_mut_ptr(), bytes.as_ptr() as *const c_void, bytes.len() as _, None);
            let mut ctx = ctx.assume_init();
            ffi::cw_unpack_next(&mut ctx);
            ffi::cw_unpack_next(&mut ctx);
            ffi::cw_unpack_next(&mut ctx);
            ffi::cw_unpack_next(&mut ctx);
            ffi::cw_unpack_next(&mut ctx);
            ffi::cw_unpack_next(&mut ctx);
            black_box(ctx.return_code);
        }
    }));
}

criterion_group!(benches, bench_pack_cwpack_rs, bench_pack_cwpack_c, bench_pack_rmp, bench_unpack_cwpack_rs, bench_unpack_cwpack_c);
criterion_main!(benches);
