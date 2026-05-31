use crate::domain::entity::RustupMirrorSource;

pub fn get_builtin_sources() -> Vec<RustupMirrorSource> {
    let sources = vec![
        ("rsproxy (字节跳动)", "https://rsproxy.cn", "https://rsproxy.cn/rustup"),
        ("ustc (中科大)", "https://mirrors.ustc.edu.cn/rust-static", "https://mirrors.ustc.edu.cn/rust-static/rustup"),
        ("tuna (清华)", "https://mirrors.tuna.tsinghua.edu.cn/rustup", "https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup"),
        ("bfsu (北外)", "https://mirrors.bfsu.edu.cn/rustup", "https://mirrors.bfsu.edu.cn/rustup/rustup"),
        ("sjtu (上交)", "https://mirrors.sjtug.sjtu.edu.cn/rust-static", "https://mirrors.sjtug.sjtu.edu.cn/rustup/rustup"),
        ("nju (南大)", "https://mirrors.nju.edu.cn/rustup", "https://mirrors.nju.edu.cn/rustup/rustup"),
        ("hust (华科)", "https://mirrors.hust.edu.cn/rustup", "https://mirrors.hust.edu.cn/rustup/rustup"),
    ];
    sources.into_iter().enumerate().map(|(i, (name, dist, update))| {
        RustupMirrorSource {
            id: format!("__builtin_{}", i + 1),
            name: name.to_string(),
            dist_server: dist.to_string(),
            update_root: update.to_string(),
            is_builtin: true,
        }
    }).collect()
}

pub fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL cannot be empty".to_string());
    }
    if !url.starts_with("https://") {
        return Err("URL must start with https://".to_string());
    }
    Ok(())
}
