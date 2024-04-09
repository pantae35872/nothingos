#![feature(os_str_display)]

use std::env;

fn main() {
    nasm_rs::compile_library_args(
        "bootlib",
        &[
            "src/boot/boot.asm",
            "src/boot/header.asm",
            "src/boot/long_mode.asm",
        ],
        &["-felf64"],
    )
    .unwrap();
    let outdir = env::var_os("OUT_DIR").expect("Out dir must set");
    println!("cargo:rustc-link-arg={}/long_mode.o", outdir.display());
    println!("cargo:rustc-link-arg={}/header.o", outdir.display());
    println!("cargo:rustc-link-arg={}/boot.o", outdir.display());
    println!("cargo:rustc-link-arg=-n");
    println!("cargo:rustc-link-arg=-T");
    println!("cargo:rustc-link-arg=linker.ld");
}
