// *****************************************************************************
//
// This program is free software; you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation; either version 2 of the License, or (at your option) any later
// version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along with
// this program; if not, write to the Free Software Foundation, Inc.,
// 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
//
// Module authors:
//   Georg Brandl <g.brandl@fz-juelich.de>
//
// *****************************************************************************

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
