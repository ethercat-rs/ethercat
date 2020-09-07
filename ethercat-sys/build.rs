// Part of ethercat-rs. Copyright 2018-2020 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

use std::{env, fmt::Write, fs, path::PathBuf};

fn main() {
    let path = env::var("ETHERCAT_PATH").expect(
        "Please set the ETHERCAT_PATH env var to the location of \
                 a checkout of the Ethercat master after running configure",
    );

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

    // Generate the EC_IOCTL_ ioctl numbers -- bindgen can't handle them.
    let code =
        fs::read_to_string(&format!("{}/master/ioctl.h", path)).expect("master/ioctl.h not found");
    let mut new = String::new();
    for line in code.split('\n') {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() >= 3
            && parts[0] == "#define"
            && parts[1].starts_with("EC_IOCTL_")
            && parts[2].starts_with("EC_IO")
        {
            let name = &parts[1]["EC_IOCTL_".len()..];

            // FIXME:
            // There are constant names that start with a number
            // so they would be invalid rust code.
            // The problem exists e.g. if you try to
            // compile the v1.5.2-sncn-11 branch of the synapticon
            // fork: https://github.com/synapticon/Etherlab_EtherCAT_Master
            // So as a first dirty workaround we just ignore them
            // to be able to compile.
            if name.starts_with("64_REF_CLK") {
                continue;
            }

            let mut numparts = parts[2].split("(");
            let access = match numparts.next().unwrap() {
                "EC_IO" => match name {
                    "SEND" | "SEND_EXT" => "arg",
                    x if x.starts_with("DOMAIN_") => "arg",
                    _ => "none",
                },
                "EC_IOR" => "read",
                "EC_IOW" => "write",
                "EC_IOWR" => "readwrite",
                _ => unreachable!("invalid IO macro found"),
            };
            let number = numparts.next().unwrap().trim_matches(&[')', ','][..]);
            let argtype = parts.get(3).map(|p| match p.trim_matches(')') {
                "uint32_t" => "u32",
                "uint64_t" => "u64",
                "size_t" => "usize",
                x => x,
            });
            write!(
                &mut new,
                "ioctl!({:10} {:20} with EC, {}{}{});\n",
                access,
                name,
                number,
                if argtype.is_some() { "; " } else { "" },
                argtype.unwrap_or("")
            )
            .unwrap();
        }
    }
    fs::write(out_path.join("ioctls.rs"), new.as_bytes())
        .expect("failed to write ioctls.rs bindings");
}
