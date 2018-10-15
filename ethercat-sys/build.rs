extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let path = env::var("ETHERCAT_PATH")
        .expect("Please set the ETHERCAT_PATH env var to the location of \
                 a checkout of the Ethercat master after running configure");

    let bindings = bindgen::Builder::default()
        .header(format!("{}/lib/ioctl.h", path))
        .clang_arg(format!("-I{}", path))
        .derive_default(true)
        .derive_debug(false)
        .prepend_enum_name(false)
        .ignore_functions()
        .whitelist_type("ec_ioctl_.*")
        .whitelist_type("ec_master_state_t")
        .whitelist_var("EC_IOCTL_.*")
        .whitelist_var("EC_MAX_.*")
        .layout_tests(false)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
