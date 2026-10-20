use crate::entry::{java_probe_file, JavaEntry};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const SEARCH_KEYWORDS: &[&str] = &[
    "java", "jdk", "env", "环境", "run", "软件", "jre", "mc", "dragon", "bin", "soft", "cache",
    "temp", "corretto", "roaming", "users", "craft", "program", "世界", "net", "游戏", "oracle",
    "game", "file", "data", "jvm", "服务", "server", "客户", "client", "整合", "应用", "运行",
    "前置", "mojang", "官启", "新建文件夹", "eclipse", "microsoft", "hotspot", "runtime", "x86",
    "x64", "forge", "原版", "optifine", "官方", "启动", "hmcl", "mod", "高清", "download",
    "launch", "程序", "path", "version", "baka", "zulu", "local", "packages",
    "4297127d64ec6", "1.", "jbr",
];

/// 搜索并校验本机 Java 安装。
pub fn search_java(install_roots: &[PathBuf]) -> Vec<JavaEntry> {
    let mut pre_list: HashMap<String, bool> = HashMap::new();

    collect_from_env(&mut pre_list);
    for root in install_roots {
        search_folder(root, &mut pre_list, false, true);
    }
    collect_platform_roots(&mut pre_list, install_roots);

    filter_special_paths(&mut pre_list);
    dedupe_case_insensitive(&mut pre_list);

    let mut entries: Vec<JavaEntry> = pre_list
        .into_iter()
        .map(|(path, user)| JavaEntry::new(path, user))
        .collect();

    check_list(&mut entries);
    entries.sort_by(|a, b| java_sorter(a, b).then_with(|| a.path_folder.cmp(&b.path_folder)));
    entries
}

fn collect_from_env(pre_list: &mut HashMap<String, bool>) {
    let path_env = std::env::var("PATH").unwrap_or_default();
    let java_home = std::env::var("JAVA_HOME").unwrap_or_default();
    for segment in format!("{path_env};{java_home}").split([';', ':']) {
        let mut path = segment.trim().trim_matches('"').to_string();
        if path.is_empty() {
            continue;
        }
        if !path.ends_with(std::path::MAIN_SEPARATOR) {
            path.push(std::path::MAIN_SEPARATOR);
        }
        if java_probe_file(Path::new(&path)).is_file() {
            pre_list.insert(path.clone(), false);
        }
        let bin = PathBuf::from(&path).join("bin");
        if java_probe_file(&bin).is_file() {
            let s = normalize_folder(&bin);
            pre_list.insert(s, false);
        }
    }
}

fn collect_platform_roots(pre_list: &mut HashMap<String, bool>, install_roots: &[PathBuf]) {
    #[cfg(windows)]
    {
        if let Ok(disks) = sys_windows_drives() {
            for disk in disks {
                search_folder(&disk, pre_list, false, false);
            }
        }
        if let Some(profile) = dirs_home() {
            search_folder(&profile.join(".jdks"), pre_list, false, true);
            if let Some(appdata) = dirs_data_local() {
                search_folder(&appdata, pre_list, false, false);
            }
            if let Some(roaming) = dirs_data_roaming() {
                search_folder(&roaming, pre_list, false, false);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        for p in [
            "/Library/Java/JavaVirtualMachines",
            "/System/Library/Java/JavaVirtualMachines",
        ] {
            scan_macos_jvm_root(Path::new(p), pre_list);
        }
        if let Some(home) = dirs_home() {
            search_folder(&home.join(".jdks"), pre_list, false, true);
            search_folder(&home.join("Library/Java/JavaVirtualMachines"), pre_list, false, true);
        }
        for p in ["/opt/homebrew/opt", "/usr/local/opt"] {
            search_folder(Path::new(p), pre_list, false, true);
        }
    }

    #[cfg(target_os = "linux")]
    {
        for p in ["/usr/lib/jvm", "/usr/java", "/opt/java", "/opt/jdk"] {
            search_folder(Path::new(p), pre_list, false, true);
        }
        if let Some(home) = dirs_home() {
            search_folder(&home.join(".jdks"), pre_list, false, true);
        }
    }

    for root in install_roots {
        if root.is_dir() {
            search_folder(root, pre_list, false, true);
        }
    }
}

#[cfg(windows)]
fn sys_windows_drives() -> Result<Vec<PathBuf>, std::io::Error> {
    use std::fs;
    let mut out = Vec::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if fs::metadata(&drive).is_ok() {
            out.push(PathBuf::from(drive));
        }
    }
    Ok(out)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(windows)]
fn dirs_data_roaming() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

#[cfg(windows)]
fn dirs_data_local() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

fn scan_macos_jvm_root(root: &Path, pre_list: &mut HashMap<String, bool>) {
    let Ok(read) = std::fs::read_dir(root) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let home_bin = path.join("Contents/Home/bin");
        if java_probe_file(&home_bin).is_file() {
            pre_list.insert(normalize_folder(&home_bin), false);
        }
        let bin = path.join("bin");
        if java_probe_file(&bin).is_file() {
            pre_list.insert(normalize_folder(&bin), false);
        }
    }
}

fn filter_special_paths(pre_list: &mut HashMap<String, bool>) {
    pre_list.retain(|k, _| {
        let norm = k.replace('\\', "/").to_lowercase();
        !(norm.contains("java8path_target_")
            || norm.contains("javapath_target_")
            || norm.contains("javatmp")
            || norm.contains("system32")
            || norm.contains(".minecraft/runtime"))
    });
}

fn dedupe_case_insensitive(pre_list: &mut HashMap<String, bool>) {
    let mut merged: HashMap<String, (String, bool)> = HashMap::new();
    for (k, v) in pre_list.drain() {
        let key = k.to_lowercase();
        merged
            .entry(key)
            .and_modify(|e| {
                if v {
                    e.1 = true;
                }
            })
            .or_insert((k, v));
    }
    pre_list.extend(merged.into_values().map(|(k, v)| (k, v)));
}

fn search_folder(path: &Path, results: &mut HashMap<String, bool>, source: bool, is_full_search: bool) {
    if let Err(e) = search_folder_inner(path, results, source, is_full_search, 0) {
        let _ = e;
    }
}

const MAX_DEPTH: u32 = 32;

fn search_folder_inner(
    original: &Path,
    results: &mut HashMap<String, bool>,
    source: bool,
    is_full_search: bool,
    depth: u32,
) -> Result<(), ()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    if !original.is_dir() {
        return Ok(());
    }

    let folder = original.to_path_buf();
    if java_probe_file(&folder).is_file() {
        results.insert(normalize_folder(&folder), source);
    }

    let mac_home = folder.join("Contents/Home/bin");
    if java_probe_file(&mac_home).is_file() {
        results.insert(normalize_folder(&mac_home), source);
    }

    let read = match std::fs::read_dir(&folder) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };

    let parent_name = folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    for entry in read.flatten() {
        let child_path = entry.path();
        if !child_path.is_dir() {
            continue;
        }
        if is_reparse_point(&child_path) {
            continue;
        }
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_lowercase();
        let should_enter = is_full_search
            || parent_name == "users"
            || name.parse::<i64>().is_ok()
            || name == "bin"
            || SEARCH_KEYWORDS.iter().any(|kw| name.contains(kw));
        if should_enter {
            search_folder_inner(&child_path, results, source, false, depth + 1)?;
        }
    }
    Ok(())
}

fn is_reparse_point(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        std::fs::symlink_metadata(path)
            .map(|m| m.file_attributes() & 0x400 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::fs::symlink_metadata(path)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
    }
}

fn normalize_folder(dir: &Path) -> String {
    let mut s = dir.to_string_lossy().into_owned();
    if !s.ends_with(std::path::MAIN_SEPARATOR) {
        s.push(std::path::MAIN_SEPARATOR);
    }
    s
}

fn check_list(entries: &mut Vec<JavaEntry>) {
    let mut ok = Vec::new();
    for entry in entries.drain(..) {
        let mut e = entry;
        match e.check() {
            Ok(()) => ok.push(e),
            Err(_) => continue,
        }
    }
    *entries = ok;
}

/// 按 64 位、JRE/JDK、版本权重排序（路径优先级由调用方 roots 决定）。
fn java_sorter(left: &JavaEntry, right: &JavaEntry) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    if left.is_64bit != right.is_64bit {
        return if left.is_64bit { Less } else { Greater };
    }
    if left.is_jre != right.is_jre {
        return if left.is_jre { Less } else { Greater };
    }
    if left.version_code() != right.version_code() {
        let wl = version_weight(left.version_code());
        let wr = version_weight(right.version_code());
        return wr.cmp(&wl);
    }
    let rl = left.version.map(|v| v.revision).unwrap_or(0);
    let rr = right.version.map(|v| v.revision).unwrap_or(0);
    (rl as i64 - 51).abs().cmp(&(rr as i64 - 51).abs())
}

fn version_weight(code: u32) -> i32 {
    const WEIGHT: [i32; 32] = [
        0, 1, 2, 3, 4, 5, 6, 14, 30, 10, 12, 15, 13, 9, 8, 7, 11, 31, 29, 16, 17, 28, 27, 26, 25,
        24, 23, 22, 21, 20, 19, 18,
    ];
    WEIGHT.get(code as usize).copied().unwrap_or(0)
}
