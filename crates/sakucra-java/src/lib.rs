mod entry;
mod search;
mod version;

pub use entry::{JavaCheckError, JavaEntry};
pub use search::search_java;
pub use version::JavaVersion;

/// 启动器安装目录与（可选）Minecraft 目录，用于优先搜索 Java。
pub fn default_search_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            roots.push(parent.to_path_buf());
        }
    }
    roots
}
