# daos-rs

`daos-rs` 是 DAOS C API 的 Rust 绑定封装，提供两种构建模式：

1. 默认模式（不启用 `mock`）：在构建阶段通过 `bindgen` 基于 `wrapper.h` 生成 `bindings.rs`，并链接系统中的 `daos` 动态库。
2. `mock` 模式：使用仓库内置的 `src/bindings.rs` 和 `src/mock_daos.rs`，不依赖本地安装 DAOS，可用于本地测试与开发联调。

## 依赖

- Rust 2021
- `bindgen`（构建依赖）
- 默认模式下需要系统可用的 DAOS 头文件与 `libdaos`

## 使用方式

作为依赖引入时（包名 `daos-rs`，库名 `daos`）：

```toml
[dependencies]
daos = { package = "daos-rs", version = "0.1.0" }
```

启用 mock：

```toml
[dependencies]
daos = { package = "daos-rs", version = "0.1.0", features = ["mock"] }
```

命令行构建示例：

```bash
# 默认模式（需要本地 DAOS 环境）
cargo build

# mock 模式（无 DAOS 环境也可构建）
cargo build --features mock
```
