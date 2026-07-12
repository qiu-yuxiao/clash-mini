// NOTE: 项目数据存储安全说明 —— 无 SQL 注入风险
//
// 经过全面代码审计，本项目不使用任何关系型数据库（SQLite/MySQL/PostgreSQL 等），
// 因此不存在传统意义上的 SQL 注入攻击面。
//
// 项目数据持久化方式：
// 1. YAML/JSON 配置文件（verge.yaml / profiles.yaml / clash.config.yaml）
//    - 使用 serde_yaml_ng / serde_json 进行序列化/反序列化
//    - 不拼接字符串构造配置内容，而是通过结构化数据操作
//    - 配置文件仅本地读写，不受外部用户直接输入控制
// 2. Tauri 内置存储（app_data_dir 等）
//    - 用于存放二进制资源、日志文件等
// 3. 系统代理设置 / 注册表（Windows）
//    - 通过 Windows API 操作，不涉及 SQL
//
// 潜在注入类风险排查结果：
// - 命令注入：clash core 启动使用 std::process::Command，参数均为内部配置，不受用户直接控制
// - 路径遍历：配置文件路径通过 dirs::app_data_dir() 生成，用户输入路径会做规范化处理
// - YAML 解析：使用 serde 的安全反序列化，不执行任意代码
//
// 未来若引入数据库：
// - 必须使用参数化查询（prepared statements）
// - 禁止字符串拼接 SQL 语句
// - 使用 ORM（如 diesel、sqlx）时应启用安全特性
mod clash;
#[allow(clippy::module_inception)]
mod config;
mod encrypt;
mod prfitem;
pub mod profiles;
pub mod runtime;
mod verge;

pub use self::{clash::*, config::*, encrypt::*, prfitem::*, profiles::*, verge::*};

pub const DEFAULT_PAC: &str = r#"function FindProxyForURL(url, host) {
  return "PROXY 127.0.0.1:%mixed-port%; SOCKS5 127.0.0.1:%mixed-port%; DIRECT;";
}
"#;
