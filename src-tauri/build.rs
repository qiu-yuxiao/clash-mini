fn main() {
    #[cfg(feature = "clippy")]
    {
        println!("cargo:warning=Skipping tauri_build during Clippy");
    }

    #[cfg(not(feature = "clippy"))]
    {
        tauri_build::build();

        // 背景：tauri-build 生成的 Windows 资源（含声明 ComCtl32 v6 的应用清单）只通过
        // `cargo:rustc-link-arg-bins` 链接到可执行文件；而 tauri 引用了仅 comctl32 v6
        // 提供的 TaskDialogIndirect，测试目标缺少清单时会退回 v5.82，测试进程以
        // STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139) 启动失败。
        //
        // 修复方式的取舍：
        // - `rustc-link-arg` 会同时作用于 bin，与 tauri-build 的 -bins 重复链接同一份
        //   resource.lib，导致 CVT1100/LNK1123 链接失败；
        // - `rustc-link-arg-tests` 只覆盖 tests/ 目标，不覆盖 lib 内单测（cargo test --lib）。
        // 因此仅在 `CLASH_MINI_TEST_LINK` 环境变量存在时启用（由 `pnpm cargo-test` 注入），
        // 日常构建与发布构建完全不受影响。
        println!("cargo:rerun-if-env-changed=CLASH_MINI_TEST_LINK");

        #[cfg(windows)]
        if std::env::var_os("CLASH_MINI_TEST_LINK").is_some() {
            if let Ok(out_dir) = std::env::var("OUT_DIR") {
                let resource_lib = std::path::Path::new(&out_dir).join("resource.lib");
                if resource_lib.exists() {
                    println!("cargo:rustc-link-arg={}", resource_lib.display());
                }
            }
        }
    }
}
