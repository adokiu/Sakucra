use crate::version::{parse_java_version_output, JavaVersion};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JavaCheckError {
    #[error("未找到 java 可执行文件")]
    NotFound,
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaEntry {
    /// java 所在 bin 目录，以路径分隔符结尾。
    pub path_folder: String,
    pub is_user_import: bool,
    #[serde(skip)]
    pub version: Option<JavaVersion>,
    pub version_string: String,
    pub is_jre: bool,
    pub is_64bit: bool,
}

impl JavaEntry {
    pub fn new(folder: impl AsRef<Path>, is_user_import: bool) -> Self {
        let mut path_folder = folder.as_ref().to_string_lossy().into_owned();
        if !path_folder.ends_with(std::path::MAIN_SEPARATOR) {
            path_folder.push(std::path::MAIN_SEPARATOR);
        }
        Self {
            path_folder,
            is_user_import,
            version: None,
            version_string: String::new(),
            is_jre: false,
            is_64bit: true,
        }
    }

    pub fn path_java(&self) -> PathBuf {
        PathBuf::from(format!("{}{}", self.path_folder, java_exe_name()))
    }

    pub fn version_code(&self) -> u32 {
        self.version.map(|v| v.version_code()).unwrap_or(0)
    }

    /// 用户可见的一行摘要。
    pub fn display_line(&self) -> String {
        let version_string = self
            .version
            .map(|v| v.display_short())
            .unwrap_or_else(|| self.version_string.clone());
        let mut code = self.version_code().to_string();
        if version_string.starts_with(&code) {
            code = version_string.clone();
        }
        format!(
            "{}{} ({}){}{}：{}",
            if self.is_jre { "JRE " } else { "JDK " },
            self.version_code(),
            code,
            if self.is_64bit { "" } else { "，32 位" },
            if self.is_user_import { "，手动导入" } else { "" },
            self.path_folder
        )
    }

    /// 运行 `java -version` 并校验该安装是否可用。
    pub fn check(&mut self) -> Result<(), JavaCheckError> {
        let path_java = self.path_java();
        if !path_java.is_file() {
            return Err(JavaCheckError::NotFound);
        }

        let folder = Path::new(&self.path_folder);
        if folder.join("pdf-bookmark").exists() {
            return Err(JavaCheckError::Message(
                "不兼容 PDF Bookmark 的 Java".into(),
            ));
        }

        self.is_jre = !folder.join(javac_exe_name()).exists();

        let output = Command::new(&path_java)
            .arg("-version")
            .output()
            .map_err(|e| JavaCheckError::Message(format!("尝试运行该 Java 失败：{e}")))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = if !stderr.is_empty() {
            stderr.into_owned()
        } else {
            stdout.into_owned()
        };

        if combined.is_empty() {
            return Err(JavaCheckError::Message("尝试运行该 Java 失败".into()));
        }

        let lower = combined.to_lowercase();
        if lower.contains("/lib/ext exists") {
            return Err(JavaCheckError::Message(
                "无法运行该 Java，请在删除 Java 文件夹中的 /lib/ext 文件夹后再试".into(),
            ));
        }
        if lower.contains("a fatal error") {
            return Err(JavaCheckError::Message(
                "无法运行该 Java，该 Java 或系统存在问题".into(),
            ));
        }

        let version = parse_java_version_output(&combined)
            .map_err(JavaCheckError::Message)?;
        self.version = Some(version);
        self.version_string = version.display_short();
        self.is_64bit = lower.contains("64-bit");

        if !self.is_64bit && is_64bit_system() {
            return Err(JavaCheckError::Message(
                "该 Java 为 32 位版本，请安装 64 位的 Java".into(),
            ));
        }

        if self.is_jre && self.version_code() >= 16 {
            return Err(JavaCheckError::Message(
                "由于高版本 JRE 对游戏的兼容性很差，因此不再允许使用。你可以使用对应版本的 JDK，而非 JRE！".into(),
            ));
        }

        Ok(())
    }
}

#[cfg(windows)]
fn java_exe_name() -> &'static str {
    "java.exe"
}

#[cfg(windows)]
fn javac_exe_name() -> &'static str {
    "javac.exe"
}

#[cfg(windows)]
fn java_probe_name() -> &'static str {
    "javaw.exe"
}

#[cfg(not(windows))]
fn java_exe_name() -> &'static str {
    "java"
}

#[cfg(not(windows))]
fn javac_exe_name() -> &'static str {
    "javac"
}

#[cfg(not(windows))]
fn java_probe_name() -> &'static str {
    "java"
}

pub fn java_probe_file(folder: &Path) -> PathBuf {
    folder.join(java_probe_name())
}

fn is_64bit_system() -> bool {
    #[cfg(target_pointer_width = "64")]
    {
        true
    }
    #[cfg(not(target_pointer_width = "64"))]
    {
        false
    }
}
