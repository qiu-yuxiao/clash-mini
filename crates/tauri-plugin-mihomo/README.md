# Tauri Plugin Mihomo

> [!IMPORTANT]
>
> 此仓库未发布到 crates.io 和 npm 中，请通过 git 使用
>
> ```shell
> # Cargo.toml
> tauri-plugin-mihomo = { git = "https://github.com/clash-verge-rev/tauri-plugin-mihomo" }
>
> # package.json
> "tauri-plugin-mihomo-api": "git+https://github.com/clash-verge-rev/tauri-plugin-mihomo"
> ```

一个基于 Tauri 框架调用 Mihomo API 的插件，支持 Mihomo 的 HTTP 和 Socket 通信

### 测试 Mimoho 所有 API 的接口状态

推荐使用 [nextest](https://github.com/nextest-rs/nextest) （一款更干净美观、速度更快的跨平台测试运行器）进行单元测试

默认使用 socket 连接 Mihomo 测试，可通过设置 `MIHOMO_SOCKET` 环境变量来使用 http 连接 Mihomo 测试

> 修改 `.env` 配置文件，将 `MIHOMO_SOCKET` 设置为 `0`, 再执行单元测试

```shell
# 此命令会排除 restart/reload_config 方法, 因为这两个接口都会让内核重新加载配置文件，会导致其他测试用例错误
cargo nextest run mihomo_

# --------------------------
# 测试 reload_config 方法
cargo nextest run reload

# 测试 restart 方法
cargo nextest run restart
```

### Contribute

##### 准备环境

- [`prek`](https://github.com/j178/prek): ⚡ Better `pre-commit`, re-engineered in Rust，**用于对 Git 提交前的检查**

#### 构建前端文件

```shell
pnpm i
pnpm build
```

#### 修改了 `model.rs` 后，如需重新导出前端对象

```shell
cargo test export_bindings
```
