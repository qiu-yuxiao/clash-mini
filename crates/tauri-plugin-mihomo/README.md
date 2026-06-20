# Tauri Plugin Mihomo

> [!IMPORTANT]
>
> This repository is not published to crates.io and npm, please use it via git:
>
> ```shell
> # Cargo.toml
> tauri-plugin-mihomo = { git = "https://github.com/clash-verge-rev/tauri-plugin-mihomo" }
>
> # package.json
> "tauri-plugin-mihomo-api": "git+https://github.com/clash-verge-rev/tauri-plugin-mihomo"
> ```

A plugin for invoking Mihomo API based on the Tauri framework, supporting both HTTP and Socket communication for Mihomo.

### Test Mihomo API Status

We recommend using [nextest](https://github.com/nextest-rs/nextest) (a cleaner, faster cross-platform test runner) for unit testing.

By default, socket connection is used for Mihomo testing. You can set the `MIHOMO_SOCKET` environment variable to use http connection instead.

> Modify the `.env` configuration file, set `MIHOMO_SOCKET` to `0`, then execute the unit tests.

```shell
# This command excludes restart/reload_config methods, as these two APIs make the kernel reload config file, causing other test cases to fail
cargo nextest run mihomo_

# --------------------------
# Test reload_config method
cargo nextest run reload

# Test restart method
cargo nextest run restart
```

### Contribute

##### Environment Setup

- [`prek`](https://github.com/j178/prek): ⚡ Better `pre-commit`, re-engineered in Rust, used for pre-commit Git hook verification.

#### Build Frontend Files

```shell
pnpm i
pnpm build
```

#### Re-export Frontend Bindings After Modifying `model.rs`

```shell
cargo test export_bindings
```
