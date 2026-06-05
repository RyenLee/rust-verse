# RustVerse 技术架构文档

> **生成时间**: 2026-05-31\
> **版本**: v1.4.2\
> **目标读者**: 项目维护者、贡献者

***

## 目录

1. [项目概述](#1-项目概述)
2. [技术栈](#2-技术栈)
3. [架构概览](#3-架构概览)
4. [领域层 (domain/)](#4-领域层-domain)
5. [应用层 (application/)](#5-应用层-application)
6. [基础设施层 (infrastructure/)](#6-基础设施层-infrastructure)
7. [接口层 (interfaces/)](#7-接口层-interfaces)
8. [根模块](#8-根模块)
9. [核心流程](#9-核心流程)
10. [错误处理策略](#10-错误处理策略)
11. [状态管理](#11-状态管理)
12. [开发者指南](#12-开发者指南)

***

## 1. 项目概述

**RustVerse** 是一个基于 Tauri 2.0 的桌面应用程序，提供 Rust 工具链的可视化版本管理。它是 `rustup` 的图形化前端，支持工具链的安装、卸载、切换，以及组件管理、Target 管理、Cargo 插件管理、镜像源切换（crm）、环境变量配置和历史版本浏览等功能。

项目采用 **领域驱动设计（DDD）** 四层架构，严格按照 **Rust 2018+ 无** **`mod.rs`** **模块规范** 组织代码，依赖方向严格向内。

## 2. 技术栈

| 层级          | 技术 / crate                                                                            |
| ----------- | ------------------------------------------------------------------------------------- |
| 桌面框架        | **Tauri 2.11** (tray-icon, image-png)                                                 |
| 运行时         | **tokio** (full features)                                                             |
| 持久化         | **redb 4.1** (嵌入式纯 Rust key-value 数据库)                                                |
| 序列化         | **serde + serde\_json**                                                               |
| 错误处理        | **thiserror 2.0**                                                                     |
| 命令行解析       | **regex, rs-histver**                                                                 |
| 跨平台二进制查找    | **which**                                                                             |
| 路径管理        | **dirs**                                                                              |
| HTTP 请求     | **reqwest 0.13** (stream feature)                                                     |
| 配置迁移        | **toml**                                                                              |
| 流处理         | **futures-util**                                                                      |
| Windows 注册表 | **winreg** (仅 Windows)                                                                |
| 更新/存储/对话框   | **tauri-plugin-updater, tauri-plugin-store, tauri-plugin-dialog, tauri-plugin-shell** |
| 前端防默认事件     | **tauri-plugin-prevent-default**                                                      |

## 3. 架构概览

项目采用 DDD 四层架构，依赖方向严格向内：

```
┌──────────────────────────────────────────────────┐
│                    lib.rs                        │
│          (依赖组装 + Tauri Builder)                │
│                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │interfaces│  │ settings │  │notificat │       │
│  │(命令层)   │  │(设置模块) │  │(通知模块) │       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘       │
│       │             │             │              │
│       ▼             ▼             ▼              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │application│ │infrastru │  │  state   │       │
│  │(应用层)   │  │cture     │  │(状态管理) │       │
│  └────┬─────┘  │(基础设施) │  └──────────┘       │
│       │        └────┬─────┘                      │
│       │             │                            │
│       ▼             ▼                            │
│  ┌──────────────────────┐                        │
│  │       domain/        │                        │
│  │   (领域层 - 零外部依赖) │                        │
│  └──────────────────────┘                        │
└──────────────────────────────────────────────────┘
```

**依赖规则**：

| 层                   | 能依赖                            | 不能依赖                              |
| ------------------- | ------------------------------ | --------------------------------- |
| **domain/**         | 无 (纯 Rust + serde)             | Tauri, redb, tokio, platform APIs |
| **application/**    | domain/                        | Tauri commands                    |
| **infrastructure/** | domain/ 系统调用                   | Tauri commands                    |
| **interfaces/**     | application/ + infrastructure/ | — (最外层)                           |

**目录结构**：

```
src-tauri/src/
├── main.rs                         # 入口点
├── lib.rs                          # 依赖组装 + Tauri Builder + 命令注册
├── state.rs                        # AppState（核心状态）
├── settings.rs                     # 用户设置模型 + 持久化
├── notification.rs                 # 通知系统（模型 + 持久化 + CRUD）
│
├── domain.rs                       # 领域层模块声明
├── domain/
│   ├── base.rs                     # 领域基础模块
│   │   └── time.rs                 # 时间工具 (chrono_now_ms)
│   ├── config_keys.rs              # 配置键名常量
│   ├── entity.rs                   # 所有领域实体
│   ├── error.rs                    # AppError / AppResult
│   ├── mirror.rs                   # 镜像名称/模式校验
│   ├── notification.rs             # 通知领域类型
│   ├── parsing.rs                  # CLI 输出纯解析函数
│   ├── repository.rs               # 数据仓库 trait 定义
│   └── settings.rs                 # 领域层设置模型
│
├── application.rs                  # 应用层模块声明
├── application/
│   ├── env_check.rs                # 环境检测（rustup/cargo 发现）
│   ├── env_var.rs                  # 环境变量元数据构建
│   ├── locale.rs                   # 语言包扫描 + TTL 缓存
│   ├── persist.rs                  # 环境变量持久化（Win注册表 / Unix shell）
│   └── rustup.rs                   # Rustup 生命周期（安装/卸载/PATH刷新/迁移）
│
├── infrastructure.rs               # 基础设施层模块声明
├── infrastructure/
│   ├── config.rs                   # 应用配置模型（从 DB 组装）
│   ├── config/
│   │   ├── defaults.rs             # 默认值定义
│   │   └── app_config.rs           # AppConfig 结构体
│   ├── db.rs                       # redb 数据库（DataStore trait 实现）
│   ├── exec.rs                     # 命令执行器（同步/流式/代理感知/取消支持）
│   ├── installer.rs                # Rustup 安装器（下载/缓存/执行）
│   ├── logger.rs                   # 文件日志（分级/轮转/运行时调整）
│   ├── notifier.rs                 # 通知事件桥接（统一发射入口）
│   ├── proxy.rs                    # 代理解析器（none/system/manual）
│   └── system.rs                   # 系统工具入口
│       └── env.rs                  # 环境变量解析 + PATH 管理
│
└── interfaces.rs                   # 接口层模块声明
    └── interfaces/commands/
        ├── component.rs            # 组件管理命令
        ├── env_check.rs            # 环境检测命令
        ├── env_var.rs              # 环境变量读写命令
        ├── histver.rs              # 历史版本浏览命令
        ├── locale.rs               # 语言包命令
        ├── mirror.rs               # 镜像源（crm）命令
        ├── override_cmd.rs         # Override 管理命令
        ├── persist.rs              # 环境变量持久化命令
        ├── plugin.rs               # Cargo 插件管理命令
        ├── target.rs               # Target 管理命令
        ├── toolchain.rs            # 工具链管理命令
        └── update.rs               # 更新 + 网络诊断命令
```

## 4. 领域层 (domain/)

> **零外部框架依赖，纯业务逻辑。**

### 4.1 `domain/entity.rs` — 领域实体

所有核心数据结构，均实现 `serde::Serialize` 以便跨 IPC 传递。

| 实体                                          | 用途                    |
| ------------------------------------------- | --------------------- |
| `ToolchainInfo`                             | 已安装工具链（名称、频道、是否默认/活跃） |
| `MirrorInfo`                                | 镜像源（名称、索引地址、类型、是否当前）  |
| `MirrorLatency`                             | 镜像网络延迟测试结果            |
| `CrmTestResult`                             | 镜像延迟测试汇总              |
| `EnvVarEntry` / `EnvVarMeta` / `EnvVarInfo` | 环境变量（元数据 / 当前值）       |
| `ComponentInfo`                             | Rust 组件（安装状态）         |
| `TargetInfo`                                | 编译目标（安装状态）            |
| `OverrideInfo`                              | 目录级工具链覆盖              |
| `CargoPluginInfo` / `SearchResult`          | Cargo 插件 / 搜索结果       |
| `HistRelease`                               | 历史 Rust 版本            |
| `UpdateInfo`                                | 工具链更新信息               |
| `NetworkDiagResult`                         | 网络诊断结果                |
| `EnvCheck` / `VersionInfo`                  | 环境检测 / 版本信息           |

### 4.2 `domain/error.rs` — 错误类型

```rust
pub enum AppError {
    Command(String),       // 命令执行失败
    Timeout(u64),          // 命令超时
    BinaryNotFound(String),// 二进制未找到
    Parse(String),         // 解析错误
    Config(String),        // 配置错误
    Network(String),       // 网络错误
}

pub type AppResult<T> = Result<T, AppError>;
```

`AppError` 实现 `Serialize`，返回结构化的 `{ kind, message }` JSON 给前端。

### 4.3 `domain/parsing.rs` — 纯解析函数

无副作用、无 I/O 的 CLI 输出解析函数：

| 函数                          | 输入                         | 输出                     |
| --------------------------- | -------------------------- | ---------------------- |
| `parse_toolchain_list()`    | `rustup toolchain list` 输出 | `Vec<ToolchainInfo>`   |
| `parse_component_list()`    | `rustup component list` 输出 | `Vec<ComponentInfo>`   |
| `parse_target_list()`       | `rustup target list` 输出    | `Vec<TargetInfo>`      |
| `parse_override_list()`     | `rustup override list` 输出  | `Vec<OverrideInfo>`    |
| `parse_cargo_plugin_list()` | `cargo install --list` 输出  | `Vec<CargoPluginInfo>` |
| `parse_search_results()`    | `cargo search` 输出          | `Vec<SearchResult>`    |
| `parse_mirror_list()`       | `crm list` 输出              | `Vec<MirrorInfo>`      |
| `parse_test_results()`      | `crm test` 输出              | `CrmTestResult`        |
| `parse_check_update()`      | `rustup check` 输出          | `Vec<UpdateInfo>`      |
| `parse_channel_from_name()` | 工具链名称                      | 频道字符串                  |

### 4.4 `domain/mirror.rs` — 镜像校验

- `validate_mirror_name(name)` — 校验镜像名称（仅允许字母、数字、`-`、`_`）
- `validate_best_mode(mode)` — 校验最佳镜像选择模式

### 4.5 `domain/repository.rs` — 数据仓库 trait

定义了领域层的数据访问契约（依赖倒置原则）：

- `ConfigRepository` — 简单 key-value 配置
- `EnvVarRepository` — 环境变量元数据
- `PluginRepository` — Cargo 插件白名单
- `NotificationRepository` — 通知 CRUD + 清理
- `SettingsRepository` — 用户设置
- `DataStore` — 组合以上所有 trait 的统一访问接口

基础设施层（`infrastructure/db.rs` 的 `RedbDataStore`）实现这些 trait。

### 4.6 `domain/settings.rs` / `domain/notification.rs`

领域层类型定义：

- `UserSettings` — 用户设置模型（minimize_to_tray / proxy / theme / notifications）
- `NotificationsConfig` — 通知分类开关 + 自动清理配置
- `Category` / `Priority` / `Notification` / `NotificationKey` — 通知领域类型
- `is_valid_locale_code()` 在 `application/locale.rs` 中

## 5. 应用层 (application/)

> **编排领域服务和基础设施，实现业务用例。仅依赖 domain/ 和 infrastructure/。**

### 5.1 `application/rustup.rs` — Rustup 生命周期

| 函数                             | 职责                                                |
| ------------------------------ | ------------------------------------------------- |
| `refresh_process_path_inner()` | 从 Windows 注册表刷新 PATH + CARGO\_HOME + RUSTUP\_HOME |
| `is_binary_functional()`       | 通过 `--version` 检测二进制是否可用                          |
| `uninstall_rustup()`           | 卸载 rustup（含文件锁定重试 + Windows 提权重试）                 |
| `install_rustup()`             | 安装 rustup（先检测是否已安装，分平台实现）                         |
| `get_webview_data_dir()`       | WebView2 用户数据目录                                   |
| `get_db_path()`                | redb 数据库文件路径 (`data/config.redb`)                 |
| `migrate_db_to_data_dir()`     | 将旧扁平位置的数据库迁移到 `data/` 目录                          |
| `try_migrate_from_toml()`      | 从旧 `config.toml` 迁移到 redb                         |

### 5.2 `application/persist.rs` — 环境变量持久化

平台特定实现，`persist_env_var()` / `remove_persisted_env_var()` / `is_env_var_persisted()` 自动分发：

| 平台          | 持久化方式                                                |
| ----------- | ---------------------------------------------------- |
| Windows     | 写入用户级注册表 (`winreg`)，移除时清理 Rust 工具链残留路径               |
| Linux/macOS | 写入 `~/.profile`（Bash/Zsh），以 `# RustVerse managed` 标记 |

### 5.3 `application/env_var.rs` — 环境变量元数据

- `build_env_var_metas_from_db()` — 从 redb 读取并按固定分类顺序构建 `EnvVarMeta` 列表
- `handle_special_env_var_set()` — 处理特定环境变量设置（如 `RUST_LOG` 时同步日志级别）
- `handle_special_env_var_remove()` — 处理特定环境变量移除

### 5.4 `application/env_check.rs` — 环境检测

- `emit_log()` — 向前端推送环境检测日志（双向：前端事件 + 文件日志）
- `check_rustup()` — 搜索并验证 rustup/cargo 二进制

### 5.5 `application/locale.rs` — 语言包管理

- `get_locales_from_config_or_db()` — 从 DB 读取语言包配置，按 TTL 过期检测，扫描 `locals/` 目录
- 使用 `Mutex<Instant>` 实现 TTL 缓存，避免重复扫描文件系统

## 6. 基础设施层 (infrastructure/)

> **实现领域接口，与系统、数据库、命令行交互。**

### 6.1 `infrastructure/db.rs` — redb 数据库 + DataStore 实现

`RedbDataStore` 结构体实现了 `DataStore` trait（及其组合的各项子 trait），是应用层的统一数据访问入口。

| 表名                | 用途                               |
| ----------------- | -------------------------------- |
| `config_simple`   | 简单 key-value（二进制路径、本地化、超时、事件、解析） |
| `config_plugins`  | 官方 Cargo 插件白名单                   |
| `config_env_vars` | 环境变量元数据                          |
| `config_settings` | 用户设置 JSON                        |
| `notifications`   | 通知数据（在 `notification.rs` 中管理）    |

核心 API：

```rust
// 生命周期
pub fn open_or_create(path: &Path) -> Result<Database, redb::Error>;

// Simple CRUD (实现 ConfigRepository)
pub fn get_simple(db: &Database, key: &str) -> Option<String>;
pub fn set_simple(db: &Database, key: &str, value: &str) -> Result<(), redb::Error>;
pub fn get_simple_batch(db: &Database, keys: &[&str]) -> HashMap<String, String>;
pub fn delete_simple(db: &Database, key: &str) -> Result<bool, redb::Error>;

// 插件管理 (实现 PluginRepository)
pub fn get_plugin_names(db: &Database) -> Option<Vec<String>>;
pub fn set_plugin_names(db: &Database, names: &[String]) -> Result<(), redb::Error>;

// 环境变量元数据 (实现 EnvVarRepository)
pub fn get_env_vars(db: &Database) -> HashMap<String, HashMap<String, EnvVarEntryConfig>>;
pub fn set_env_var_entry(db: &Database, category: &str, name: &str, entry: &EnvVarEntryConfig) -> Result<(), redb::Error>;
pub fn delete_env_var_entry(db: &Database, category: &str, name: &str) -> Result<bool, redb::Error>;

// 用户设置 (实现 SettingsRepository)
pub fn get_settings_json(db: &Database) -> Option<String>;
pub fn set_settings_json(db: &Database, json: &str) -> Result<(), redb::Error>;

// 通知管理 (实现 NotificationRepository)
// notification_ensure_table / notification_insert / notification_list
// notification_mark_read / notification_mark_unread
// notification_delete / notification_delete_all / notification_unread_count
// notification_delete_read_before — 清理过期已读通知
```

配置辅助函数从 DB 读取组装结构化配置：

```rust
pub fn get_binaries_config(db: &Database) -> (String, String);
pub fn get_parsing_config(db: &Database) -> ParsingConfig;
pub fn get_events_config(db: &Database) -> EventsConfig;
```

### 6.2 `infrastructure/config.rs` — 配置模型

`AppConfig` 及子结构体，所有字段从 redb 组装，带 `Default` 回退：

- `AppMetadataConfig` — 应用元数据
- `BinariesConfig` — 二进制路径
- `PathsConfig` — 路径配置
- `LocaleConfig` — 语言包配置
- `TimeoutsConfig` — 超时配置
- `EventsConfig` — 事件名称配置
- `PluginsConfig` — 插件配置
- `ParsingConfig` — 解析标记配置
- `EnvVarsConfig` + `EnvVarEntryConfig` — 环境变量配置

`get_config()` 作为 `#[tauri::command]` 注册供前端获取完整配置。

### 6.3 `infrastructure/exec.rs` — 命令执行器

```rust
// 同步执行（捕获输出，带超时）
pub async fn run_command(bin: &str, args: &[&str], timeout_secs: u64) -> AppResult<String>;

// 允许特定退出码（如 rustup check 的 100）
pub async fn run_command_with_timeout_allow_codes(
    bin: &str, args: &[&str], timeout_secs: u64, allowed_codes: &[i32]
) -> AppResult<String>;

// 流式执行 + 取消支持（安装/更新用）
pub async fn run_command_with_cancel(
    app: AppHandle, bin: &str, args: &[&str],
    locale_key: &str, log_event: &str, finished_event: &str,
    timeout_secs: u64, cancel_flag: Arc<AtomicBool>,
) -> AppResult<()>;

// 流式执行 + 取消 + 重试（更新用）
pub async fn run_command_with_cancel_retry(
    app: AppHandle, bin: &str, args: &[&str],
    locale_key: &str, log_event: &str, finished_event: &str,
    max_retries: u32, retry_delay_ms: u64, timeout_secs: u64,
    cancel_flag: Arc<AtomicBool>,
) -> AppResult<()>;
```

- 所有命令执行前强制设置 `LC_ALL=C` 确保输出格式一致，设置 `CARGO_HTTP_MULTIPLEXING=false` 避免连接复用问题
- 自动注入 `RUSTUP_DIST_SERVER` / `RUSTUP_UPDATE_ROOT` 镜像环境变量
- 全量应用代理配置（通过 `proxy.rs` 获取当前代理设置）
- Windows 平台设置 `CREATE_NO_WINDOW` 避免弹出终端窗口
- `run_command_with_cancel` 逐行读取 stdout/stderr，通过 `app.emit()` 实时推送进度，同时轮询 `cancel_flag` 支持前端取消

### 6.4 `infrastructure/logger.rs` — 日志系统

- 全局单例 `FileLogger`（`OnceLock` + `Mutex`）
- 支持 5 级日志：`Trace < Debug < Info < Warn < Error`
- 运行时可通过 Tauri 命令动态调整日志级别
- 自动轮转：单文件上限 5MB，保留最近 5 个文件
- 日志目录：`[exe_dir]/logs/`
- 前端日志可通过 `frontend_log` 命令回写到后端文件

### 6.5 `infrastructure/proxy.rs` — 代理管理

- 三种模式：`none`（直连）、`system`（系统代理）、`manual`（手动配置）
- 全局 `ProxyConfig` 缓存，支持 `invalidate_cache()` 强制刷新
- `apply_proxy_env()` 将代理设置为命令的 HTTP\_PROXY/HTTPS\_PROXY 环境变量

### 6.6 `infrastructure/installer.rs` — 安装器管理

- 下载 Rustup 官方安装器（根据平台 + 架构选择 URL）
- 缓存到 `[exe_dir]/data/` 目录
- 下载进度通过 Tauri Event 推送到前端
- 支持最多 3 次重试

### 6.7 `infrastructure/system/env.rs` — 系统环境

- `resolve_rust_homes()` — 解析 CARGO\_HOME / RUSTUP\_HOME
- `find_binary()` — 在 PATH 中查找二进制
- `binary_exists()` — 检查二进制是否存在
- `validate_rust_binary()` — 校验二进制路径包含 `rustup`/`cargo` 字样
- Windows 特定：`read_system_env_var()`, `read_user_env_var()` — 读取注册表

## 7. 接口层 (interfaces/)

> **极薄 Tauri 命令适配器层，无业务逻辑。**

每个命令函数遵循统一模式：

1. 反序列化输入参数
2. 委托到 `application/` 或 `domain/` 服务
3. 处理错误条件
4. 返回格式化结果

### 7.1 命令列表

| 命令                         | 模块             | 功能                 |
| -------------------------- | -------------- | ------------------ |
| `check_env`                | `env_check`    | 检测 Rust 运行环境       |
| `get_versions`             | `env_check`    | 获取 Rustup/Cargo 版本 |
| `list_toolchains`          | `toolchain`    | 列出已安装工具链           |
| `install_toolchain`        | `toolchain`    | 安装工具链（流式输出）        |
| `uninstall_toolchain`      | `toolchain`    | 卸载工具链              |
| `set_default_toolchain`    | `toolchain`    | 设置默认工具链            |
| `get_override`             | `override_cmd` | 获取目录覆盖             |
| `set_override`             | `override_cmd` | 设置目录覆盖             |
| `remove_override`          | `override_cmd` | 移除目录覆盖             |
| `list_overrides`           | `override_cmd` | 列出所有覆盖             |
| `list_components`          | `component`    | 列出组件               |
| `add_component`            | `component`    | 添加组件               |
| `remove_component`         | `component`    | 移除组件               |
| `list_targets`             | `target`       | 列出 Target          |
| `add_target`               | `target`       | 添加 Target          |
| `remove_target`            | `target`       | 移除 Target          |
| `check_update`             | `update`       | 检查工具链更新            |
| `update_all`               | `update`       | 更新所有工具链            |
| `update_rustup`            | `update`       | 更新 Rustup 自身       |
| `diag_network`             | `update`       | 网络诊断               |
| `list_cargo_plugins`       | `plugin`       | 列出已安装插件            |
| `search_plugins`           | `plugin`       | 搜索 Cargo 插件        |
| `install_plugin`           | `plugin`       | 安装 Cargo 插件        |
| `uninstall_plugin`         | `plugin`       | 卸载 Cargo 插件        |
| `list_env_vars`            | `env_var`      | 列出环境变量             |
| `get_env_var`              | `env_var`      | 获取环境变量             |
| `set_env_var`              | `env_var`      | 设置环境变量             |
| `remove_env_var`           | `env_var`      | 移除环境变量             |
| `update_env_var_meta`      | `env_var`      | 更新环境变量元数据          |
| `delete_env_var_meta`      | `env_var`      | 删除环境变量元数据          |
| `persist_env_var`          | `persist`      | 持久化环境变量            |
| `remove_persisted_env_var` | `persist`      | 移除持久化              |
| `is_env_var_persisted`     | `persist`      | 查询持久化状态            |
| `list_persisted_env_vars`  | `persist`      | 列出所有持久化变量          |
| `get_locale`               | `locale`       | 获取当前语言包            |
| `set_locale`               | `locale`       | 切换语言包              |
| `list_available_locales`   | `locale`       | 列出可用语言包            |
| `check_crm_installed`      | `mirror`       | 检测 crm 是否安装        |
| `install_crm`              | `mirror`       | 安装 crm             |
| `crm_list`                 | `mirror`       | 列出镜像源              |
| `crm_current`              | `mirror`       | 当前镜像源              |
| `crm_version`              | `mirror`       | crm 版本             |
| `crm_use`                  | `mirror`       | 切换镜像源              |
| `crm_best`                 | `mirror`       | 最佳镜像源              |
| `crm_default`              | `mirror`       | 恢复默认               |
| `crm_test`                 | `mirror`       | 测试镜像延迟             |
| `sync_hist_releases`       | `histver`      | 同步历史版本             |
| `list_hist_releases`       | `histver`      | 列出历史版本             |
| `search_hist_releases`     | `histver`      | 搜索历史版本             |
| `count_hist_releases`      | `histver`      | 统计历史版本             |
| `install_rustup`           | `lib.rs`       | 安装 Rustup          |
| `uninstall_rustup`         | `lib.rs`       | 卸载 Rustup          |
| `refresh_process_path`     | `lib.rs`       | 刷新进程 PATH          |
| `get_log_dir`              | `lib.rs`       | 获取日志目录             |
| `frontend_log`             | `lib.rs`       | 前端日志回写             |
| `get_log_level`            | `lib.rs`       | 获取日志级别             |
| `set_log_level`            | `lib.rs`       | 设置日志级别             |
| `get_config`               | `lib.rs`       | 获取完整配置             |
| `get_settings`             | `lib.rs`       | 获取用户设置             |
| `save_settings`            | `lib.rs`       | 保存用户设置             |
| `notify_*` (8个)            | `notification` | 通知系统 CRUD          |
| `cleanup_expired_notifications`| `lib.rs`    | 手动清理过期已读通知        |
| `is_background_task_running`  | `lib.rs`       | 查询后台任务运行状态       |
| `cancel_background_task`      | `lib.rs`       | 取消后台任务              |

## 8. 根模块

### 8.1 `lib.rs` — 依赖组装

核心职责：

1. **模块声明**：对所有顶层模块进行 `mod` 声明
2. **Tauri 命令定义**：直接定义部分命令（安装/卸载/日志/设置/通知），其余自 `interfaces/commands/` 导入
3. **`run()`** **入口**：
   - 初始化日志系统
   - 迁移数据库位置和格式（`migrate_db_to_data_dir` + `try_migrate_from_toml`）
   - 打开/创建 redb 数据库
   - 创建 `AppState` 和 `LocaleScanState`
   - 初始化代理解析器
   - 构建 Tauri Builder：注册所有插件 + 命令 + 状态
   - 系统托盘：显示/退出菜单 + 关闭最小化到托盘

### 8.2 `state.rs` — 应用状态

```rust
pub struct AppState {
    pub rustup_path: Mutex<Option<PathBuf>>,
    pub cargo_path: Mutex<Option<PathBuf>>,
    pub store: Arc<dyn DataStore>,  // trait-based, supports multi-backend
    pub task_state: BackgroundTaskState,
    pub locale: Mutex<String>,
}

pub struct BackgroundTaskState {
    pub running: Mutex<bool>,            // single-task-at-a-time enforcement
    pub cancel_flag: Arc<AtomicBool>,    // async cancellation signalling
}
```

- `AppState` 通过 `tauri::Builder::manage()` 注入，每个命令通过 `State<'_, AppState>` 获取。
- `DataStore` 是组合 trait，统一了 `ConfigRepository`、`EnvVarRepository`、`PluginRepository`、`NotificationRepository`、`SettingsRepository`，通过 `Arc<dyn DataStore>` 注入，支持多后端切换。
- `BackgroundTaskState` 确保安装/更新任务互斥（`running` 标志），并提供异步取消信号（`cancel_flag`）。
- `LocaleScanState` 独立管理，通过 `.manage()` 注入，专门用于语言包扫描的 TTL 缓存。

### 8.3 `settings.rs` — 用户设置

`UserSettings` 包含：

- `minimize_to_tray` — 关闭时最小化到托盘
- `proxy_type` / `proxy_host` / `proxy_port` — 代理配置
- `notifications` — 通知分类开关
- `theme` — 主题（auto/dark/light）

所有字段默认值均为安全状态（全关）。保存前先校验，保存后立即验证写入持久性。

### 8.4 `notification.rs` — 通知系统

独立的通知子系统，位于 `src-tauri/src/notification.rs`，包含通知的持久化操作（插入、查询、标记已读/未读、删除、清理过期通知）。

- **持久层**: 操作 redb 的 `notifications` 表，使用自增 u64 ID。
- **事件桥接**: `infrastructure/notifier.rs` 是实现通知发射的统一入口，由业务模块调用。
  - fire-and-forget 模式：写入失败不阻塞调用方业务逻辑。
  - 偏好感知：检查 `NotificationsConfig`（enabled / 分类开关 / DND）后决定是否推送实时事件。
  - i18n 分离：后端只存储 `notif_key` + `params_json`，前端通过 `vue-i18n` 解析。
  - 双通道：所有通知持久化（历史可查），仅用户允许时才推送 `notification:new` Tauri 事件（Toast/桌面通知）。
- **自动清理**: `lib.rs` 启动后台定时器，每 60 秒检查设置中的 `auto_cleanup_minutes`，自动删除过期已读通知。同时，`save_settings` 命令保存后立即触发一次清理。

## 9. 核心流程

### 9.1 工具链列表查询

```
前端调用 list_toolchains(rustup_path)
    │
    ▼
interfaces/commands/toolchain.rs
    ├── 校验 rustup_path 安全性 (validate_rust_binary)
    ├── 从 DB 读取解析配置 (get_parsing_config)
    ├── 执行 rustup toolchain list (infrastructure/exec::run_command)
    └── 解析输出 (domain/parsing::parse_toolchain_list)
            │
            ▼
        返回 Vec<ToolchainInfo>
```

### 9.2 工具链安装（流式）

```
前端调用 install_toolchain(app, state, rustup_path, channel, date)
    │
    ▼
interfaces/commands/toolchain.rs
    ├── 校验二进制 + 构建工具链名称
    ├── 从 DB 读取事件名配置 + 语言包
    └── 委托 infrastructure/exec::run_command_with_streaming()
            │
            ▼
infrastructure/exec.rs
    ├── 启动 rustup toolchain install <name>
    ├── 应用代理设置
    ├── stdout/stderr 逐行读取
    ├── 通过 app.emit(log_event) 向前端推送进度
    └── 完成后 emit(finished_event)
```

### 9.3 用户设置保存

```
前端调用 save_settings(state, settings)
    │
    ▼
lib.rs save_settings()
    ├── settings::save_settings_inner()
    │   ├── UserSettings::validate() — 校验所有字段
    │   ├── 序列化为 JSON
    │   ├── infrastructure::db::set_settings_json() — 写入 redb
    │   └── 读回验证持久性
    └── infrastructure::proxy::invalidate_cache() — 刷新代理缓存
```

### 9.4 环境变量持久化（Windows）

```
前端调用 persist_env_var(state, key, value)
    │
    ▼
application/persist::persist_env_var()
    ├── 校验名称合法性
    ├── 调用 persist_env_var_windows()
    │   ├── winreg 写入 HKCU\Environment
    │   └── 广播 WM_SETTINGCHANGE 通知系统
    └── 同时调用 set_env_var() 更新当前进程
```

### 9.5 语言包查询（带 TTL 缓存）

```
前端调用 list_available_locales(state, scan_state, force_refresh)
    │
    ▼
application/locale::get_locales_from_config_or_db()
    ├── 检查 TTL 缓存；未过期则直接返回
    ├── 从 DB 读取 locale.codes 和 locale.meta
    ├── 扫描文件系统 locals/ 目录
    ├── 乱码过滤（is_valid_locale_code）
    ├── 更新缓存
    └── 返回 LocaleInfo 列表
```

## 10. 错误处理策略

- **领域层**定义 `AppError` 枚举（6 种变体），使用 `thiserror` derive
- 所有 Tauri 命令返回 `Result<T, AppError>`
- `AppError` 手动实现 `Serialize`，向前端返回 `{ kind, message }` 结构
- Tauri 自动将 `Result::Err` 转换为前端可接收的错误对象
- 关键路径的错误通过 `logger::logger().error()` 记录到文件
- 数据库操作失败时有 in-memory 回退策略

## 11. 状态管理

| 状态                | 类型             | 管理方式                         | 用途                      |
| ----------------- | -------------- | ---------------------------- | ----------------------- |
| `AppState`        | `tauri::State` | `.manage(app_state)`         | rustup/cargo 路径 + DataStore + 后台任务状态 |
| `LocaleScanState` | `tauri::State` | `.manage(locale_scan_state)` | 语言包 TTL 缓存              |
| `DataStore`       | `Arc<dyn DataStore>`| `AppState.store`        | 统一数据访问（redb / JSON 等后端） |
| `BackgroundTaskState`| `AppState`  | `AppState.task_state`       | 后台任务互斥 + 取消信号          |
| `FileLogger`      | 全局单例           | `OnceLock`                   | 日志记录                    |
| `ProxyConfig`     | 全局缓存           | `OnceLock<Mutex>`            | 代理配置缓存                  |
| `LogLevel`        | 全局变量           | `OnceLock<Mutex<LogLevel>>`  | 运行时日志级别                 |

## 12. 开发者指南

### 12.1 构建

```powershell
# 开发模式
pnpm tauri dev

# 生产构建
pnpm tauri build

# 仅检查 Rust 编译
cd src-tauri
cargo check

# 运行测试
cargo test
```

### 12.2 添加新命令

1. 在 `interfaces/commands/` 创建 `my_feature.rs`
2. 在 `interfaces/commands.rs` 中添加 `pub mod my_feature;`
3. 编写 `#[tauri::command]` 函数，遵循薄适配器模式
4. 在 `lib.rs` 中导入并注册到 `generate_handler![]`

### 12.3 添加新领域逻辑

1. **纯解析/校验函数** → 添加到 `domain/parsing.rs` 或 `domain/mirror.rs`
2. **新实体** → 添加到 `domain/entity.rs`
3. **用例编排** → 在 `application/` 创建新模块
4. **数据库操作** → 在 `infrastructure/db.rs` 添加新的表定义和 CRUD 函数

### 12.4 依赖方向检查清单

- `domain/` 中不能出现 `use tauri::`, `use tokio::`, `use redb::`, `use crate::infrastructure::`
- `application/` 中不能出现 `#[tauri::command]`
- `infrastructure/` 中不能出现 `#[tauri::command]`
- `interfaces/commands/` 中的函数体应只有参数提取 + 委托调用 + 错误处理

### 12.5 代码规范

- Rust edition 2024
- 使用 Rust 2018+ 无 `mod.rs` 模块管理
- 编译时无 error，仅允许 `dead_code` / `unused` 等无害 warning
- 所有公开类型实现 `Serialize` 以便 IPC 传输
- 命名遵循 Rust 惯例：函数 `snake_case`，类型 `PascalCase`
- `unsafe` 块必须附注释说明原因

