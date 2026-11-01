// カーネルをリンカスクリプトでリンクする。
// (RUSTFLAGS を手で渡さなくても `cargo build -p kernel --target x86_64-unknown-none` で済む)
fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg-bins=-T{}/linker.ld", dir);
    println!("cargo:rustc-link-arg-bins=-n");
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=src/boot.s");
    println!("cargo:rerun-if-changed=build.rs");
}
