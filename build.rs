fn main() {
    cc::Build::new()
        .file("original/src/cwpack.c")
        .include("original/src")
        .compile("cwpack");
    println!("cargo:rerun-if-changed=original/src/cwpack.c");
}
