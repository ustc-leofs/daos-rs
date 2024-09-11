#![allow(non_upper_case_globals)]
#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub mod mock_daos;

use std::hash::{Hash, Hasher};

#[test]
fn test() {
    println!("HelloWorld!");
    println!("{}", DAOS_API_VERSION_MINOR);
}
