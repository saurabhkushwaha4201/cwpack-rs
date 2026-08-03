use cwpack_rs::pack::PackContext;
use cwpack_rs::unpack::UnpackContext;
use std::hint::black_box;
use std::time::Instant;
use std::io::Cursor;

fn bench_pack_cwpack_rs(iters: u32) -> std::time::Duration {
    let mut buf = [0u8; 1024];
    let start = Instant::now();
    for _ in 0..iters {
        let mut ctx = PackContext::new(&mut buf);
        ctx.pack_array_size(3);
        ctx.pack_unsigned(black_box(42));
        ctx.pack_str(black_box(b"hello world"));
        ctx.pack_map_size(1);
        ctx.pack_str(black_box(b"key"));
        ctx.pack_boolean(black_box(true));
        
        let n = unsafe { ctx.current.offset_from(ctx.start) } as usize;
        black_box(buf[n - 1]);
    }
    start.elapsed()
}

fn bench_pack_rmp(iters: u32) -> std::time::Duration {
    let mut buf = Vec::with_capacity(1024);
    let start = Instant::now();
    for _ in 0..iters {
        buf.clear();
        rmp::encode::write_array_len(&mut buf, 3).unwrap();
        rmp::encode::write_uint(&mut buf, black_box(42)).unwrap();
        rmp::encode::write_str(&mut buf, black_box("hello world")).unwrap();
        rmp::encode::write_map_len(&mut buf, 1).unwrap();
        rmp::encode::write_str(&mut buf, black_box("key")).unwrap();
        rmp::encode::write_bool(&mut buf, black_box(true)).unwrap();
        black_box(buf[buf.len() - 1]);
    }
    start.elapsed()
}

fn bench_unpack_cwpack_rs(iters: u32) -> std::time::Duration {
    let mut buf = [0u8; 1024];
    let mut pack_ctx = PackContext::new(&mut buf);
    pack_ctx.pack_array_size(3);
    pack_ctx.pack_unsigned(42);
    pack_ctx.pack_str(b"hello world");
    pack_ctx.pack_map_size(1);
    pack_ctx.pack_str(b"key");
    pack_ctx.pack_boolean(true);
    let n = unsafe { pack_ctx.current.offset_from(pack_ctx.start) } as usize;
    let bytes = &buf[..n];

    let start = Instant::now();
    for _ in 0..iters {
        let mut ctx = UnpackContext::new(bytes);
        ctx.unpack_next();
        ctx.unpack_next();
        ctx.unpack_next();
        ctx.unpack_next();
        ctx.unpack_next();
        ctx.unpack_next();
        black_box(ctx.return_code);
    }
    start.elapsed()
}

fn bench_unpack_rmp(iters: u32) -> std::time::Duration {
    let mut buf = [0u8; 1024];
    let mut pack_ctx = PackContext::new(&mut buf);
    pack_ctx.pack_array_size(3);
    pack_ctx.pack_unsigned(42);
    pack_ctx.pack_str(b"hello world");
    pack_ctx.pack_map_size(1);
    pack_ctx.pack_str(b"key");
    pack_ctx.pack_boolean(true);
    let n = unsafe { pack_ctx.current.offset_from(pack_ctx.start) } as usize;
    let bytes = &buf[..n];

    let start = Instant::now();
    for _ in 0..iters {
        let mut cur = Cursor::new(bytes);
        let _ = rmp::decode::read_array_len(&mut cur);
        let _ = rmp::decode::read_int::<u32, _>(&mut cur);
        let l = rmp::decode::read_str_len(&mut cur).unwrap() as u64; cur.set_position(cur.position() + l);
        let _ = rmp::decode::read_map_len(&mut cur);
        let l = rmp::decode::read_str_len(&mut cur).unwrap() as u64; cur.set_position(cur.position() + l);
        let ret = rmp::decode::read_bool(&mut cur);
        black_box(ret);
    }
    start.elapsed()
}

fn main() {
    let iters = 10_000_000;
    println!("Running {} iterations...", iters);
    
    let d = bench_pack_cwpack_rs(iters);
    println!("Pack cwpack-rs: {} ms", d.as_millis());
    
    let d = bench_pack_rmp(iters);
    println!("Pack rmp: {} ms", d.as_millis());
    
    let d = bench_unpack_cwpack_rs(iters);
    println!("Unpack cwpack-rs: {} ms", d.as_millis());
    
    let d = bench_unpack_rmp(iters);
    println!("Unpack rmp: {} ms", d.as_millis());
}
