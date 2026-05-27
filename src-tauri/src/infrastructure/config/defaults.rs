//! 应用默认值 —— 所有配置项的 fallback 值集中定义于此。
//!
//! 从 `db.rs` 提取出来的纯函数，不依赖数据库。被 `AppConfig::default()`、
//! `get_*_config()` 批量读取器和初始数据播种共同使用。

use std::collections::HashMap;

use super::app_config::EnvVarEntryConfig;

// ── App metadata ──

fn config_toml_app_field(field: &str) -> Option<String> {
    let content = include_str!("../../../config.toml");
    let value: toml::Value = toml::from_str(content).ok()?;
    value.get("app")?.get(field)?.as_str().map(|s| s.to_string())
}

pub fn app_name() -> String {
    config_toml_app_field("name").unwrap_or_else(|| "RustVerse".to_string())
}
pub fn app_version() -> String {
    config_toml_app_field("version").unwrap_or_else(|| "1.0.0".to_string())
}
pub fn app_description() -> String {
    config_toml_app_field("description")
        .unwrap_or_else(|| "Rust Toolchain Visual Version Manager".to_string())
}

// ── Binaries ──

pub fn rustup() -> String {
    "rustup".to_string()
}
pub fn cargo() -> String {
    "cargo".to_string()
}

// ── Paths ──

pub fn cargo_bin_relative() -> String {
    ".cargo/bin".to_string()
}

// ── Locale ──

pub fn force_locale() -> String {
    "C".to_string()
}

// ── Timeouts ──

pub fn cargo_search_seconds() -> u64 {
    30
}
pub fn rustup_check_seconds() -> u64 {
    30
}

// ── Events ──

pub fn install_log() -> String {
    "install-log".to_string()
}
pub fn install_finished() -> String {
    "install-finished".to_string()
}
pub fn plugin_install_log() -> String {
    "plugin-install-log".to_string()
}
pub fn plugin_install_finished() -> String {
    "plugin-install-finished".to_string()
}
pub fn update_log() -> String {
    "update-log".to_string()
}
pub fn update_finished() -> String {
    "update-finished".to_string()
}

// ── Parsing ──

pub fn default_marker() -> String {
    "(default)".to_string()
}
pub fn active_marker() -> String {
    "(active)".to_string()
}
pub fn installed_marker() -> String {
    "(installed)".to_string()
}
pub fn no_overrides() -> String {
    "no overrides".to_string()
}
pub fn up_to_date() -> String {
    "Up to date".to_string()
}
pub fn update_available() -> String {
    "Update available".to_string()
}
pub fn version_separator() -> String {
    " -> ".to_string()
}
pub fn status_separator() -> String {
    " - ".to_string()
}
pub fn cargo_prefix() -> String {
    "cargo-".to_string()
}

// ── Plugin names ──

pub fn plugin_names() -> Vec<String> {
    vec![
        "cargo-clippy".to_string(),
        "cargo-fmt".to_string(),
        "cargo-miri".to_string(),
        "cargo-rustdoc".to_string(),
        "cargo-test-fixture".to_string(),
        "rustfmt".to_string(),
        "clippy".to_string(),
        "miri".to_string(),
    ]
}

// ── Env vars ──

macro_rules! env_var_entry {
    ($rec:expr, $def:expr, $description:expr, $notes:expr) => {
        EnvVarEntryConfig {
            rec: $rec.map(|s: &str| s.to_string()),
            def: $def.map(|s: &str| s.to_string()),
            description: $description.to_string(),
            notes: $notes.to_string(),
        }
    };
}

pub fn env_vars() -> HashMap<String, HashMap<String, EnvVarEntryConfig>> {
    let mut result = HashMap::new();

    // ── 基础路径与缓存优化 ──
    let mut paths_cache = HashMap::new();
    paths_cache.insert(
        "CARGO_HOME".to_string(),
        env_var_entry!(
            None,
            Some("%USERPROFILE%\\.cargo"),
            "Cargo 家目录（存放 registry、git 仓库、已编译 crate 等）",
            "推荐挪到非系统盘，避免 C 盘膨胀。重开终端或移动目录后生效。"
        ),
    );
    paths_cache.insert(
        "RUSTUP_HOME".to_string(),
        env_var_entry!(
            None,
            Some("%USERPROFILE%\\.rustup"),
            "rustup 工具链和全局配置的安装位置",
            "推荐挪到非系统盘。需配合移动现有目录或首次安装时设置。"
        ),
    );
    paths_cache.insert("CARGO_TARGET_DIR".to_string(), env_var_entry!(
        None, None,
        "统一存放所有项目的编译输出（target 目录）",
        "可避免每个项目生成独立 target 文件夹，节省磁盘并共享编译缓存。不同项目间可能因 feature 差异偶尔需要清理。"
    ));
    paths_cache.insert("CARGO_CACHE_RUSTC_INFO".to_string(), env_var_entry!(
        Some("1"), None,
        "缓存 rustc 信息以加速下一次编译（nightly 功能）",
        "仅当使用 nightly 工具链时有效，稳定版暂不支持。"
    ));
    result.insert("paths_cache".to_string(), paths_cache);

    // ── 网络与代理 ──
    let mut network_proxy = HashMap::new();
    network_proxy.insert("HTTP_PROXY".to_string(), env_var_entry!(
        None, Some("http://127.0.0.1:7890"),
        "为 Cargo 和 rustup 指定 HTTP 代理",
        "格式 http://127.0.0.1:7890（根据本机代理地址填写）。Windows 上通常大写即可，部分工具可能同时需要小写变量。"
    ));
    network_proxy.insert("HTTPS_PROXY".to_string(), env_var_entry!(
        None, Some("https://127.0.0.1:7890"),
        "为 Cargo 和 rustup 指定 HTTPS 代理",
        "与 HTTP_PROXY 类似，用于 HTTPS 连接，值通常相同。"
    ));
    network_proxy.insert("NO_PROXY".to_string(), env_var_entry!(
        None, Some("localhost,127.0.0.1,.local"),
        "跳过代理的地址列表",
        "避免内部通信走代理，多个地址用逗号分隔。"
    ));
    network_proxy.insert("CARGO_HTTP_CAINFO".to_string(), env_var_entry!(
        None, None,
        "指定自定义 CA 证书包（如公司自签证书）",
        "指向 PEM 格式证书文件路径，解决企业环境 SSL 验证问题。"
    ));
    network_proxy.insert("CARGO_HTTP_CHECK_REVOKE".to_string(), env_var_entry!(
        None, Some("true"),
        "控制 Cargo 是否检查 SSL 证书吊销状态",
        "当遇到 SSL error 且确信网络无问题时，可临时设为 false。不推荐长期禁用，存在安全风险。"
    ));
    network_proxy.insert("CARGO_NET_RETRY".to_string(), env_var_entry!(
        None, Some("3"),
        "网络请求失败重试次数",
        "网络不稳定时可适当增大。"
    ));
    network_proxy.insert("CARGO_HTTP_TIMEOUT".to_string(), env_var_entry!(
        None, Some("30"),
        "HTTP 请求超时时间（秒）",
        "慢速网络环境建议调大，避免误报超时。"
    ));
    result.insert("network_proxy".to_string(), network_proxy);

    // ── 编译性能与缓存加速 ──
    let mut build_perf = HashMap::new();
    build_perf.insert("RUSTC_WRAPPER".to_string(), env_var_entry!(
        None, Some("sccache"),
        "在调用 rustc 前先执行指定程序（常用于 sccache）",
        "安装 sccache 后设置，需确保 sccache.exe 在 PATH 中。可通过 scoop install sccache 安装。"
    ));
    build_perf.insert("SCCACHE_DIR".to_string(), env_var_entry!(
        None, Some("%LOCALAPPDATA%\\Mozilla\\sccache"),
        "sccache 缓存存储目录",
        "建议放到空间较大的磁盘，集中管理缓存。"
    ));
    build_perf.insert("RUSTFLAGS".to_string(), env_var_entry!(
        None, Some("-C link-arg=-fuse-ld=lld"),
        "传递给 rustc 的额外编译标志（加速链接）",
        "使用 LLD 链接器可显著加快链接速度。MSVC 工具链下需配合 -C linker=rust-lld，更推荐在项目 .cargo/config.toml 中针对 target 配置，避免全局环境变量冲突。"
    ));
    build_perf.insert("CARGO_INCREMENTAL".to_string(), env_var_entry!(
        None, Some("1"),
        "启用/禁用增量编译",
        "默认开启，一般无需修改。设为 0 可关闭，在 CI 场景下可能减少磁盘消耗。"
    ));
    build_perf.insert("CARGO_JOBS".to_string(), env_var_entry!(
        None, Some("(CPU 逻辑核心数)"),
        "并行编译任务数",
        "默认等于 CPU 逻辑核心数，虚拟机或内存紧张时可调小。"
    ));
    result.insert("build_perf".to_string(), build_perf);

    // ── 调试与诊断 ──
    let mut debug_diag = HashMap::new();
    debug_diag.insert("RUST_BACKTRACE".to_string(), env_var_entry!(
        None, Some("1"),
        "控制 panic 时的回溯输出",
        "开发时建议设为 1 或 full，能显示完整调用栈。full 包含内联帧信息。"
    ));
    debug_diag.insert("RUST_LOG".to_string(), env_var_entry!(
        None, Some("debug"),
        "控制 Rust 生态工具（如 rustup、cargo、rustc）的日志级别",
        "按需设置，如 RUST_LOG=cargo::ops::resolve=trace 仅打印依赖解析日志，用于排查问题。"
    ));
    debug_diag.insert("RUSTFLAGS_DEBUG".to_string(), env_var_entry!(
        None, Some("-C debuginfo=2"),
        "生成完整调试信息（用于调试 release 模式）",
        "若需调试 release 模式，可在 RUSTFLAGS 中加入此标志。注意与其他 RUSTFLAGS 设置合并使用。"
    ));
    debug_diag.insert("CARGO_TERM_COLOR".to_string(), env_var_entry!(
        None, Some("auto"),
        "终端输出颜色",
        "默认自动检测，可强制设为 always 或 never。"
    ));
    result.insert("debug_diag".to_string(), debug_diag);

    // ── 其他实用变量 ──
    let mut misc = HashMap::new();
    misc.insert("CARGO_BUILD_TARGET".to_string(), env_var_entry!(
        None, Some("x86_64-pc-windows-msvc"),
        "指定默认编译目标",
        "当需要交叉编译或固定目标平台时使用。"
    ));
    misc.insert("RUSTUP_DIST_SERVER".to_string(), env_var_entry!(
        None, Some("https://static.rust-lang.org"),
        "自定义 rustup 工具链下载源",
        "用于镜像加速下载工具链，国内用户推荐设置为中科大或清华镜像。"
    ));
    misc.insert("RUSTUP_UPDATE_ROOT".to_string(), env_var_entry!(
        None, Some("https://static.rust-lang.org/rustup"),
        "自定义 rustup 升级服务器",
        "用于镜像加速 rustup 自身升级。"
    ));
    misc.insert("EDITOR".to_string(), env_var_entry!(
        None, Some("code.cmd"),
        "某些 Rust 工具（如 cargo config --edit）调用的编辑器",
        "可设为 code.cmd (VS Code)、notepad++.exe 等可执行程序。"
    ));
    misc.insert("VISUAL".to_string(), env_var_entry!(
        None, Some("code.cmd"),
        "类似 EDITOR，某些工具优先读取 VISUAL",
        "作用与 EDITOR 相同，但优先级可能更高，建议与 EDITOR 设为一致。"
    ));
    result.insert("misc".to_string(), misc);

    result
}