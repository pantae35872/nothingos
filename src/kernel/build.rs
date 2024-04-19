#![feature(os_str_display)]

use std::env;

fn main() {
    nasm_rs::compile_library_args("bootlib", &["src/boot/boot.asm"], &["-felf64"]).unwrap();
    //let outdir = env::var_os("OUT_DIR").expect("Out dir must set");
    //println!("cargo:rustc-link-arg={}/boot.o", outdir.display());
    //println!("cargo:rustc-link-arg=-n");
    let outdir = env::var_os("OUT_DIR").expect("Out dir must set");
    println!("cargo:rustc-link-arg={}/boot.o", outdir.display());
    println!("cargo:rustc-link-arg=-T");
    println!("cargo:rustc-link-arg=linker.ld");
}
