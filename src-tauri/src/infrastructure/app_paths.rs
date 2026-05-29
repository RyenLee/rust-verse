//! 应用路径集中管理模块
//!
//! 将所有硬编码路径收敛到 `AppPaths` 结构体，通过 `config.toml` 的 `[paths]`、`[database]` 段
//! 支持自定义路径和数据库配置覆盖。全局单例，启动时初始化一次，后续通过 [`app_paths`] 访问。
//!
//! # 配置方式
//!
//! 在 `config.toml` 中取消注释即可覆盖默认值：
//!
//! ```toml
//! [paths]
//! data_dir = "$EXE_DIR/data"
//! log_dir = "$EXE_DIR/logs"
//! webview_dir = "$EXE_DIR/webview"
//!
//! [database]
//! db_type = "redb"
//! db_name = "rustverse_db"
//! ```
//!
//! 支持的变量替换：
//! - `$EXE_DIR` — 可执行文件所在目录
//! - `$HOME` — 用户主目录
//! - `~/` — 用户主目录前缀
//!
//! # 路径派生关系
//!
//! ```text
//! exe_dir (可执行文件目录)
//! ├── data_dir    ← config.toml 可覆盖，默认 $EXE_DIR/data
//! │   ├── db_path            = data_dir/{db_name}.{db_type}  (如 rustverse_db.redb)
//! │   ├── installer_cache_dir = data_dir (与 data_dir 相同)
//! │   └── locale_config_path = data_dir/locale.json
//! ├── log_dir     ← config.toml 可覆盖，默认 $EXE_DIR/logs
//! └── webview_dir ← config.toml 可覆盖，默认 $EXE_DIR/webview
//! ```

use std::path::PathBuf;
use std::sync::OnceLock;

pub struct AppPaths {
    exe_dir: PathBuf,
    data_dir: PathBuf,
    log_dir: PathBuf,
    webview_dir: PathBuf,
    temp_dir: PathBuf,
    db_path: PathBuf,
    db_name: String,
    db_type: String,
    installer_cache_dir: PathBuf,
    locale_config_path: PathBuf,
}

static APP_PATHS: OnceLock<AppPaths> = OnceLock::new();

struct PathsOverrides {
    data_dir: Option<PathBuf>,
    log_dir: Option<PathBuf>,
    webview_dir: Option<PathBuf>,
    temp_dir: Option<PathBuf>,
}

impl Default for PathsOverrides {
    fn default() -> Self {
        Self {
            data_dir: None,
            log_dir: None,
            webview_dir: None,
            temp_dir: None,
        }
    }
}

struct DbConfig {
    db_name: String,
    db_type: String,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            db_name: "rustverse_db".to_string(),
            db_type: "redb".to_string(),
        }
    }
}

impl AppPaths {
    pub fn init() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let toml_value = Self::load_toml(&exe_dir);

        let custom = Self::parse_paths_overrides(&toml_value, &exe_dir);
        let db_cfg = Self::parse_db_config(&toml_value);

        let data_dir = custom.data_dir.unwrap_or_else(|| exe_dir.join("data"));
        let log_dir = custom.log_dir.unwrap_or_else(|| exe_dir.join("logs"));
        let webview_dir = custom
            .webview_dir
            .unwrap_or_else(|| exe_dir.join("webview"));
        let temp_dir = custom.temp_dir.unwrap_or_else(|| exe_dir.join("temp"));

        std::fs::create_dir_all(&data_dir).ok();
        std::fs::create_dir_all(&log_dir).ok();
        std::fs::create_dir_all(&webview_dir).ok();
        std::fs::create_dir_all(&temp_dir).ok();

        let db_path = data_dir.join(format!("{}.{}", db_cfg.db_name, db_cfg.db_type));
        let installer_cache_dir = data_dir.clone();
        let locale_config_path = data_dir.join("locale.json");

        Self {
            exe_dir,
            data_dir,
            log_dir,
            webview_dir,
            temp_dir,
            db_path,
            db_name: db_cfg.db_name,
            db_type: db_cfg.db_type,
            installer_cache_dir,
            locale_config_path,
        }
    }

    fn load_toml(exe_dir: &PathBuf) -> Option<toml::Value> {
        let toml_path = exe_dir.join("config.toml");
        if !toml_path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&toml_path).ok()?;
        toml::from_str(&content).ok()
    }

    fn parse_paths_overrides(value: &Option<toml::Value>, exe_dir: &PathBuf) -> PathsOverrides {
        let paths = match value.as_ref().and_then(|v| v.get("paths")) {
            Some(p) => p,
            None => return PathsOverrides::default(),
        };
        PathsOverrides {
            data_dir: paths
                .get("data_dir")
                .and_then(|v| v.as_str())
                .map(|s| shellexpand_path(s, exe_dir)),
            log_dir: paths
                .get("log_dir")
                .and_then(|v| v.as_str())
                .map(|s| shellexpand_path(s, exe_dir)),
            webview_dir: paths
                .get("webview_dir")
                .and_then(|v| v.as_str())
                .map(|s| shellexpand_path(s, exe_dir)),
            temp_dir: paths
                .get("temp_dir")
                .and_then(|v| v.as_str())
                .map(|s| shellexpand_path(s, exe_dir)),
        }
    }

    fn parse_db_config(value: &Option<toml::Value>) -> DbConfig {
        let db = match value.as_ref().and_then(|v| v.get("database")) {
            Some(d) => d,
            None => return DbConfig::default(),
        };
        let db_name = db
            .get("db_name")
            .and_then(|v| v.as_str())
            .unwrap_or("rustverse_db")
            .to_string();
        let db_type = db
            .get("db_type")
            .and_then(|v| v.as_str())
            .unwrap_or("redb")
            .to_string();
        DbConfig { db_name, db_type }
    }

    pub fn exe_dir(&self) -> &PathBuf {
        &self.exe_dir
    }
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
    pub fn log_dir(&self) -> &PathBuf {
        &self.log_dir
    }
    pub fn webview_dir(&self) -> &PathBuf {
        &self.webview_dir
    }
    pub fn temp_dir(&self) -> &PathBuf {
        &self.temp_dir
    }
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }
    pub fn db_name(&self) -> &str {
        &self.db_name
    }
    pub fn db_type(&self) -> &str {
        &self.db_type
    }
    pub fn installer_cache_dir(&self) -> &PathBuf {
        &self.installer_cache_dir
    }
    pub fn locale_config_path(&self) -> &PathBuf {
        &self.locale_config_path
    }
}

fn shellexpand_path(s: &str, exe_dir: &PathBuf) -> PathBuf {
    let expanded = s.replace("$EXE_DIR", &exe_dir.to_string_lossy()).replace(
        "$HOME",
        &dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    );
    if expanded.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&expanded[2..])
        } else {
            PathBuf::from(expanded)
        }
    } else {
        PathBuf::from(expanded)
    }
}

pub fn app_paths() -> &'static AppPaths {
    APP_PATHS.get_or_init(AppPaths::init)
}

pub fn init_global() -> &'static AppPaths {
    APP_PATHS.get_or_init(AppPaths::init)
}
