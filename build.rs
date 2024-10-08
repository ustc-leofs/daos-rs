extern crate bindgen;

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // 判断是否启用了 `mock` 特性
    let is_mock = env::var("CARGO_FEATURE_MOCK").is_ok();

    // 获取输出目录
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    if is_mock {
        // 如果启用了 `mock` 特性，拷贝现有的 bindings.rs 文件
        let src_bindings = PathBuf::from("src/bindings.rs");
        let dest_bindings = out_path.join("bindings.rs");

        fs::copy(&src_bindings, &dest_bindings).expect("Failed to copy mock bindings.rs");

        println!("cargo:rerun-if-changed=src/bindings.rs");
    } else {
        // 如果没有启用 `mock` 特性，使用 bindgen 生成绑定并链接 daos 库
        println!("cargo:rustc-link-lib=daos");

        let bindings = bindgen::Builder::default()
            .header("wrapper.h")
            .generate()
            .expect("Unable to generate bindings");

        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Couldn't write bindings!");
    }
}
