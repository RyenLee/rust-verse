# RustVerse 功能实现进度清单

> **生成时间**: 2026-05-31
> **参考版本**: v1.4.0
> **扫描范围**: 完整代码库（Rust 后端 + Vue 前端）

---

## 总体评估

| 维度 | 状态 |
|------|------|
| **后端核心功能** | 基本完整（44 个 Tauri 命令已注册） |
| **DDD 架构迁移** | 进行中但**未集成** — 新架构代码存在但未被 `lib.rs` 编译 |
| **前端页面** | 15 个页面全部实现 |
| **前端组件** | 18 个通用组件全部实现 |
| **设置/通知系统** | 前端已实现，**后端命令缺失** |
| **系统托盘** | **完全缺失** |

> **关键发现**：代码库存在两套后端架构并存的情况——旧扁平架构（`commands/`）在 `lib.rs` 中实际编译运行，新模式 DDD 四层架构（`interfaces/commands/`、`domain/`、`application/`、`infrastructure/`）的文件已写好但**未通过 `mod` 声明接入 `lib.rs`**，属于死代码。

---

## 1. 仪表盘 (Dashboard)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-DASH-01 | 显示 Rust 环境状态（rustup/cargo 是否已安装） | `DashboardView.vue` | `check_env` ✓ | **完整** |
| F-DASH-02 | 显示 Rustup 和 Cargo 版本号 | `DashboardView.vue` | `get_versions` ✓ | **完整** |
| F-DASH-03 | 显示默认工具链和已安装工具链数量 | `DashboardView.vue` | `list_toolchains` ✓ | **完整** |
| F-DASH-04 | 显示可用更新数量 | `DashboardView.vue` | `check_update` ✓ | **完整** |
| F-DASH-05 | 一键安装 Rust 环境（rustup） | `DashboardView.vue` | `install_rustup` ✓ | **完整** |
| F-DASH-06 | 安装进度实时显示（流式输出） | `ProgressDialog.vue` | `install_rustup` (流式) ✓ | **完整** |
| F-DASH-07 | 支持安装任务取消 | 前端事件 ✓ | `run_command_with_cancel` ✓ | **完整** |
| F-DASH-08 | 支持卸载 Rust 环境 | `DashboardView.vue` | `uninstall_rustup` ✓ | **完整** |
| F-DASH-09 | 支持"最小化到托盘"模式关闭窗口 | 设置项已有 | **后端托盘代码缺失** | **不完整** [3] |

---

## 2. 工具链管理 (Toolchains)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-TC-01 | 列出所有已安装工具链 | `ToolchainListView.vue` | `list_toolchains` ✓ | **完整** |
| F-TC-02 | 安装新工具链 | `ToolchainListView.vue` | `install_toolchain` ✓ | **完整** |
| F-TC-03 | 卸载工具链 | `ToolchainListView.vue` | `uninstall_toolchain` ✓ | **完整** |
| F-TC-04 | 设置默认工具链 | `ToolchainListView.vue` | `set_default_toolchain` ✓ | **完整** |
| F-TC-05 | 安装进度实时显示（流式输出） | `ProgressDialog.vue` | `install_toolchain` (流式) ✓ | **完整** |
| F-TC-06 | 支持安装任务取消 | 前端事件 ✓ | `run_command_with_cancel` ✓ | **完整** |
| F-TC-07 | 日期版工具链显示实际版本号 | 前端展示 ✓ | `display_name` 字段计算 ✓ | **完整** |
| F-TC-08 | 卸载/切换默认工具链操作触发通知 | 前端监听 | **旧代码无通知集成** | **不完整** [1] |

---

## 3. 历史版本 (History Versions)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-HIST-01 | 从 GitHub Pages 同步 Rust 历史版本列表 | `HistoryVersionView.vue` | `sync_hist_releases` ✓ | **完整** |
| F-HIST-02 | 按频道筛选历史版本 | 前端过滤 ✓ | `channel` 参数 ✓ | **完整** |
| F-HIST-03 | 搜索历史版本（按版本号） | `SearchInput.vue` | `search_hist_releases` ✓ | **完整** |
| F-HIST-04 | 分页加载历史版本 | 前端分页 ✓ | `list_hist_releases` + offset ✓ | **完整** |
| F-HIST-05 | 显示版本发布日期 | 前端展示 ✓ | `HistRelease.date` ✓ | **完整** |

---

## 4. 组件管理 (Components)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-COMP-01 | 列出工具链可用组件 | `ComponentsView.vue` | `list_components` ✓ | **完整** |
| F-COMP-02 | 安装/移除组件 | `ComponentsView.vue` | `add_component` / `remove_component` ✓ | **完整** |
| F-COMP-03 | 选择目标工具链进行组件管理 | `ToolchainSelector.vue` | `toolchain` 参数 ✓ | **完整** |
| F-COMP-04 | 搜索组件名称 | `SearchInput.vue` | 前端过滤 | **完整** |

---

## 5. 编译目标 (Targets)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-TGT-01 | 列出可用编译目标 | `TargetsView.vue` | `list_targets` ✓ | **完整** |
| F-TGT-02 | 安装/移除编译目标 | `TargetsView.vue` | `add_target` / `remove_target` ✓ | **完整** |
| F-TGT-03 | 搜索目标名称 | `SearchInput.vue` | 前端过滤 | **完整** |

---

## 6. 目录覆盖 (Overrides)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-OVR-01 | 设置目录级别的工具链覆盖 | `OverrideView.vue` | `set_override` ✓ | **完整** |
| F-OVR-02 | 列出所有覆盖配置 | `OverrideView.vue` | `list_overrides` ✓ | **完整** |
| F-OVR-03 | 移除覆盖配置 | `OverrideView.vue` | `remove_override` ✓ | **完整** |

---

## 7. 工具链更新 (Updates)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-UPD-01 | 检查所有已安装工具链的可用更新 | `UpdateView.vue` | `check_update` ✓ | **完整** |
| F-UPD-02 | 显示当前版本和可用新版本 | `UpdateView.vue` | `UpdateInfo` 结构体 ✓ | **完整** |
| F-UPD-03 | 一键更新所有工具链 | `UpdateView.vue` | `update_all` ✓ | **完整** |
| F-UPD-04 | 更新 Rustup 自身 | `UpdateView.vue` | `update_rustup` ✓ | **完整** |
| F-UPD-05 | 更新进度实时显示（流式输出） | `ProgressDialog.vue` | `update_all` (流式) ✓ | **完整** |
| F-UPD-06 | 支持更新任务取消 | 前端事件 ✓ | `cancel_flag` 机制 ✓ | **完整** |
| F-UPD-07 | 失败自动重试 | — | `max_retries` / `retry_delay_ms` ✓ | **完整** |
| F-UPD-08 | 网络诊断 | `UpdateView.vue` | `diag_network` ✓ | **完整** |

---

## 8. Cargo 插件 (Plugins)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-PLUG-01 | 列出已安装 Cargo 插件 | `PluginsView.vue` | `list_cargo_plugins` ✓ | **完整** |
| F-PLUG-02 | 搜索 Cargo 插件 | `PluginsView.vue` | `search_plugins` ✓ | **完整** |
| F-PLUG-03 | 安装 Cargo 插件 | `PluginsView.vue` | `install_plugin` ✓ | **完整** |
| F-PLUG-04 | 卸载 Cargo 插件 | `PluginsView.vue` | `uninstall_plugin` ✓ | **完整** |
| F-PLUG-05 | 安装进度实时显示 | `ProgressDialog.vue` | `install_plugin` (流式) ✓ | **完整** |

---

## 9. 环境变量 (Environment Variables)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-ENV-01 | 显示 Rust 相关环境变量 | `EnvVarsView.vue` | `list_env_vars` ✓ | **完整** |
| F-ENV-02 | 设置/修改当前进程环境变量 | `EnvVarsView.vue` | `set_env_var` ✓ | **完整** |
| F-ENV-03 | 移除环境变量 | `EnvVarsView.vue` | `remove_env_var` ✓ | **完整** |
| F-ENV-04 | Windows：持久化到注册表 | `EnvVarsView.vue` | `persist_env_var` ✓ | **完整** |
| F-ENV-05 | Linux/macOS：持久化到 ~/.profile | `EnvVarsView.vue` | `persist_env_var` ✓ | **完整** |
| F-ENV-06 | 以分类标签展示环境变量 | `EnvVarsView.vue` | 按 category 分组 ✓ | **完整** |
| F-ENV-07 | 显示环境变量元数据 | `EnvVarsView.vue` | `EnvVarMeta` 结构体 ✓ | **完整** |
| F-ENV-08 | 设置 RUST_LOG 时自动同步后端日志级别 | — | `handle_special_env_var_set` | **需验证** [2] |
| F-ENV-09 | 刷新进程 PATH 从 Windows 注册表 | — | `refresh_process_path` ✓ | **完整** |

---

## 10. 镜像源管理 (Mirrors)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-MIR-01 | 检测 crm 是否已安装 | `MirrorView.vue` | `check_crm_installed` ✓ | **完整** |
| F-MIR-02 | 一键安装 crm | `MirrorView.vue` | `install_crm` ✓ | **完整** |
| F-MIR-03 | 列出可用镜像源 | `MirrorView.vue` | `crm_list` ✓ | **完整** |
| F-MIR-04 | 切换镜像源 | `MirrorView.vue` | `crm_use` ✓ | **完整** |
| F-MIR-05 | 测试镜像延迟 | `MirrorView.vue` | `crm_test` ✓ | **完整** |
| F-MIR-06 | 自动选择最佳镜像 | `MirrorView.vue` | `crm_best` ✓ | **完整** |
| F-MIR-07 | 恢复默认镜像 | `MirrorView.vue` | `crm_default` ✓ | **完整** |
| F-MIR-08 | 显示当前镜像源 | `MirrorView.vue` | `crm_current` ✓ | **完整** |
| F-MIR-09 | 显示 crm 版本 | `MirrorView.vue` | `crm_version` ✓ | **完整** |

---

## 11. 通知系统 (Notifications) ⚠️ 高风险

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-NOTIF-01 | 操作事件实时通知 | `NotificationCenter.vue` 监听 `notification:new` | **notifier.rs 未编译** | **不完整** [1] |
| F-NOTIF-02 | 通知中心页面展示通知历史 | `NotificationCenter.vue` ✓ | **`notify_list` 未注册** | **不完整** [1] |
| F-NOTIF-03 | 通知分类 | 前端 `Category` 枚举 ✓ | `domain::notification::Category` ✓ | 数据模型就绪 |
| F-NOTIF-04 | 通知优先级 | 前端 `Priority` 枚举 ✓ | `domain::notification::Priority` ✓ | 数据模型就绪 |
| F-NOTIF-05 | 按分类筛选通知 | 前端筛选 ✓ | 前端侧处理 | 前端就绪 |
| F-NOTIF-06 | 已读/未读状态管理 | 前端 UI ✓ | **`notify_mark_read/unread` 未注册** | **不完整** [1] |
| F-NOTIF-07 | 通知点击跳转到相关页面 | `action_route` 字段 ✓ | 数据字段已定义 | **完整** |
| F-NOTIF-08 | 删除单个/全部通知 | 前端 UI ✓ | **`notify_delete/delete_all` 未注册** | **不完整** [1] |
| F-NOTIF-09 | i18n 多语言通知内容 | 前端 `$t()` + `notif_key` ✓ | `NotificationKey` 枚举已定义 | **完整** |
| F-NOTIF-10 | 高优先级 desktop 通知 | — | **未实现** | **缺失** |
| F-NOTIF-11 | 实时推送通知（toast 弹窗） | `Toast.vue` ✓ | **`notification:new` 事件无发射源** | **不完整** [1] |
| F-NOTIF-12 | 设置页面管理通知分类开关 | `SettingsView.vue` ✓ | **`save_settings` 未注册** | **不完整** [1] |
| F-NOTIF-13 | 通知数量统计（铃铛角标） | `TopBar.vue` ✓ | **`notify_unread_count` 未注册** | **不完整** [1] |
| F-NOTIF-14 | 已启用通知类型数量气泡 | — | — | **未实现** |
| F-NOTIF-15 | 自动清理过期已读通知 | 设置 UI ✓ | **`notification_delete_read_before` 未注册** | **不完整** [1] |
| F-NOTIF-16 | 免打扰模式 | 设置 UI ✓ | **`save_settings` 未注册** | **不完整** [1] |

---

## 12. 应用更新 (App Update)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-APPUPD-01 | 启动后自动检查应用更新 | 前端 `useAppUpdater` ✓ | Tauri Plugin 内置 | **完整** |
| F-APPUPD-02 | 显示可用更新版本和说明 | `AppUpdateView.vue` ✓ | Tauri Plugin 提供 | **完整** |
| F-APPUPD-03 | 下载并安装更新 | `useAppUpdater` ✓ | Tauri Plugin 提供 | **完整** |
| F-APPUPD-04 | 更新进度显示 | `AppUpdateView.vue` ✓ | Tauri Plugin 提供 | **完整** |
| F-APPUPD-05 | 手动检查更新 | `useAppUpdater.checkForUpdate` ✓ | Tauri Plugin 提供 | **完整** |

---

## 13. 用户设置 (Settings) ⚠️ 高风险

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-SET-01 | 设置应用主题 | `SettingsView.vue` (radio) | **`save_settings` 未注册** | **不完整** [1] |
| F-SET-02 | 代理设置 | `SettingsView.vue` (radio/input) | **`save_settings` 未注册** | **不完整** [1] |
| F-SET-03 | 关闭窗口时最小化到托盘 | `SettingsView.vue` (switch) | **托盘代码缺失** + 命令未注册 | **不完整** [1][3] |
| F-SET-04 | 通知分类开关 | `SettingsView.vue` (switches) | **`save_settings` 未注册** | **不完整** [1] |
| F-SET-05 | 通知默认优先级设置 | `SettingsView.vue` (radio) | **`save_settings` 未注册** | **不完整** [1] |
| F-SET-06 | 免打扰模式开关 | `SettingsView.vue` (switch) | **`save_settings` 未注册** | **不完整** [1] |
| F-SET-07 | 桌面通知声音开关 | `SettingsView.vue` (switch) | **`save_settings` 未注册** | **不完整** [1] |
| F-SET-08 | 自动清理过期通知时间 | `SettingsView.vue` (button group) | **`save_settings` 未注册** | **不完整** [1] |
| F-SET-09 | 设置项独立保存反馈 | 前端 per-item state ✓ | **命令未注册** | **不完整** [1] |

---

## 14. 帮助面板 (Help)

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-HELP-01 | 入门指南说明 | `HelpPanel.vue` ✓ | 纯前端 | **完整** |
| F-HELP-02 | 各功能模块使用说明 | `HelpPanel.vue` ✓ | 纯前端 | **完整** |
| F-HELP-03 | 关于信息 | `HelpPanel.vue` ✓ | `env!("CARGO_PKG_VERSION")` | **完整** |
| F-HELP-04 | 项目主页链接 | `HelpPanel.vue` ✓ | 纯前端 | **完整** |
| F-HELP-05 | GitHub 仓库链接 | `HelpPanel.vue` ✓ | 纯前端 | **完整** |

---

## 15. 系统功能

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-SYS-01 | 系统托盘图标和菜单 | — | **完全缺失** | **缺失** [3] |
| F-SYS-02 | 多语言支持（中文/英文） | vue-i18n ✓ | `get_locale` / `set_locale` ✓ | **完整** |
| F-SYS-03 | 运行时切换语言 | `TopBar.vue` 语言切换 ✓ | `set_locale` ✓ | **完整** |
| F-SYS-04 | 侧边栏折叠/展开 | `App.vue` toggleSidebar ✓ | 纯前端 | **完整** |
| F-SYS-05 | 快捷键 Ctrl+B 折叠侧边栏 | `handleKeydown` ✓ | 纯前端 | **完整** |
| F-SYS-06 | 窗口边界记忆 | `useAppStore.restoreWindowBounds` ✓ | 纯前端 | **完整** |
| F-SYS-07 | 应用日志记录 | `frontend_log` 命令 ✓ | 文件日志 + 分级 ✓ | **完整** |
| F-SYS-08 | 运行时调整日志级别 | RUST_LOG 联动 | `handle_special_env_var_set` | **需验证** [2] |
| F-SYS-09 | 启动界面（Splash Screen） | `SplashScreen.vue` ✓ | 纯前端 + 异步加载 | **完整** |
| F-SYS-10 | 欢迎页（未安装 rustup 引导页） | `WelcomeView.vue` ✓ | 纯前端 | **完整** |

---

## 16. 数据迁移

| ID | 需求 | 前端 | 后端 | 状态 |
|----|------|------|------|------|
| F-MIG-01 | 从旧 config.toml 迁移到 redb | — | `try_migrate_from_toml` ✓ | **完整** |
| F-MIG-02 | 从旧位置迁移数据库到 data/ | — | `migrate_db_to_data_dir` ✓ | **完整** |

---

## 17. 架构问题 — DDD 迁移未完成

架构文档描述的 DDD 四层架构代码已经编写但**未被编译集成**。文件位置对比：

| 层 | 文档描述文件 | 实际编译文件 | 状态 |
|----|------------|------------|------|
| 领域层 | `domain/entity.rs`, `domain/error.rs` 等 | `commands/*.rs`（内联实体） | 旧代码在用 |
| 应用层 | `application/rustup.rs`, `application/persist.rs` 等 | `lib.rs`（内联） | 旧代码在用 |
| 基础设施层 | `infrastructure/db.rs`, `infrastructure/exec.rs` 等 | `db.rs`, `utils/exec.rs` 等 | 旧代码在用 |
| 接口层 | `interfaces/commands/*.rs` | `commands/*.rs` | 旧代码在用 |

**关键差异**：新 DDD `interfaces/commands/` 代码引用的 `AppState` 包含 `store`（`Arc<dyn DataStore>`）和 `task_state`（`TaskState`）字段，但实际编译的 [state.rs](file:///d:/Dev/workspace/2026/05/Rust/rustverse/src-tauri/src/state.rs) 中 `AppState` 仅包含 `rustup_path`、`cargo_path`、`db` 三个字段。

---

## 问题分类汇总

| 编号 | 类别 | 影响范围 | 缺失项数 | 严重程度 |
|------|------|----------|----------|----------|
| [1] | **后端命令缺失** | 通知系统完整链路、设置持久化 | ~20 个命令 + 事件发射 | **严重** |
| [2] | **功能待验证** | RUST_LOG 日志同步 | 2 项 | 中 |
| [3] | **功能完全缺失** | 系统托盘、desktop 通知 | 2 项 | 中 |
| [4] | **架构债务** | DDD 迁移未完成 | 50+ 文件 | 高 |

---

## 优先修复建议

### 紧急（P0）

1. **注册设置持久化命令** — 前端 `SettingsView` 调用 `save_settings` 和 `get_settings` 但后端无响应，设置页面完全不可用
2. **注册通知命令** — 前端 `NotificationCenter` 调用 `notify_list/create/delete/...` 全部失败
3. **接入通知事件桥** — 编译 `infrastructure/notifier.rs`（或等同代码），否则工具链/更新操作完成后前端收不到通知

### 重要（P1）

4. **集成 DDD 架构** — 在 `lib.rs` 中声明 `mod domain;` `mod application;` `mod infrastructure;` `mod interfaces;`，让新架构编译并逐步替换旧代码
5. **实现系统托盘** — 添加 `tauri::tray::TrayIconBuilder`，至少支持显示/退出菜单项，并实现 `minimize_to_tray` 设置项逻辑
6. **通知系统端到端贯通** — 完成操作事件 → notifier → DB 写入 → `notification:new` 事件推送的完整链路

### 计划中（P2）

7. **实现 desktop 原生通知** — 高优先级通知发送系统级通知
8. **已启用通知类型数量气泡** — 通知设置摘要提示
9. **清理未编译的 DDD 死代码或完成迁移** — 避免维护两套代码

---

> 扫描工具: 静态代码分析
> 下次更新: 随迭代同步