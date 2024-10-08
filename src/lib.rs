#![allow(non_upper_case_globals)]
#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

mod mock_daos;
use std::ptr::null_mut;

#[cfg(feature = "mock")]
pub use mock_daos::*;

#[test]
fn test() {
    println!("HelloWorld!");
    println!("{}", DAOS_API_VERSION_MINOR);
    #[cfg(feature = "mock")]
    unsafe {
        let x = daos_handle_t { cookie: 0 };
        daos_array_close(x, null_mut());
        mock_test();
    }
}
