// Part of ethercat-rs. Copyright 2018-2022 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

use crate::types::{Error, Result};
use std::io;
use std::ffi::CStr;
use std::os::raw::c_char;

pub(crate) fn string_to_foe_name(input: &str) -> Result<[c_char; 32]> {
    if input.len() > 32 {
        let e = io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "FoE name can have a maximum length of 32, '{}' has {}",
                input,
                input.len()
            ),
        );
        return Err(Error::Io(e));
    }
    let mut foe_name: [std::os::raw::c_char; 32] = [0; 32];
    input
        .as_bytes()
        .iter()
        .zip(&mut foe_name)
        .for_each(|(i, r)| *r = *i as _);
    Ok(foe_name)
}

#[test]
fn test_string_to_foe_name() {
    let cmp = |s: String, chars: [i8; 32]| {
        let arr: Vec<i8> = s.as_bytes().iter().map(|c| *c as i8).collect();
        assert_eq!(chars.len(), 32);
        assert_eq!(chars[0..arr.len()], arr);
        assert_eq!(chars[arr.len()..], vec![0; 32 - s.len()]);
    };

    let name = String::from("some Name of a FoE file");
    let chars = string_to_foe_name(&name).expect("Name is ok");
    cmp(name, chars);

    let name = String::from("short");
    let chars = string_to_foe_name(&name).expect("Name is ok");
    cmp(name, chars);

    let name = String::from("\u{2665}\u{1F494};");
    let chars = string_to_foe_name(&name).expect("Name is ok");
    cmp(name, chars);

    let name = String::from("a name that is just too long so we'll see what happens");
    let e = string_to_foe_name(&name).unwrap_err();
    assert_eq!(
        e.to_string(),
        format!(
            "FoE name can have a maximum length of 32, '{}' has {}",
            name,
            name.len()
        )
    );
}

pub(crate) fn c_array_to_string(data: *const i8) -> String {
    unsafe { CStr::from_ptr(data).to_string_lossy().into_owned() }
}

#[test]
fn test_c_array_to_string() {
    let arr: [i8; 64] = [0_i8; 64];
    assert_eq!(c_array_to_string(arr.as_ptr()), "");

    let mut arr: [i8; 64] = [0_i8; 64];
    [80_i8, 114, 111, 100, 117, 99, 116, 32, 99, 111, 100, 101]
        .iter()
        .enumerate()
        .for_each(|(idx, v)| {
            arr[idx] = *v;
        });
    assert_eq!(c_array_to_string(arr.as_ptr()), "Product code");
}
