extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=ethercat");
    println!("cargo:rustc-link-search=native=/opt/etherlab/lib");

    let bindings = bindgen::Builder::default()
        .header("/home/gbr/devel/ext/ethercat-hg/lib/ioctl.h")
        .clang_arg("-I/home/gbr/devel/ext/ethercat-hg")
        .derive_default(true)
        .prepend_enum_name(false)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
