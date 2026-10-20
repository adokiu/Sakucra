use std::cmp::Ordering;

/// 四段式 Java 版本号（major.minor.build.revision）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JavaVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub revision: u32,
}

impl JavaVersion {
    pub fn version_code(&self) -> u32 {
        self.minor
    }

    pub fn display_short(&self) -> String {
        let mut s = format!("{}.{}.{}.{}", self.major, self.minor, self.build, self.revision);
        if s.starts_with("1.") {
            s = s[2..].to_string();
        }
        s
    }

    pub fn cmp_dotnet(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.build, self.revision).cmp(&(
            other.major,
            other.minor,
            other.build,
            other.revision,
        ))
    }
}

/// 从 `java -version` 输出解析版本。
pub fn parse_java_version_output(output: &str) -> Result<JavaVersion, String> {
    let lower = output.to_lowercase();
    let version_re = regex::Regex::new(r#"version "([^"]+)""#).unwrap();
    let openjdk_re = regex::Regex::new(r"openjdk (\d+)").unwrap();

    let mut version_string = version_re
        .captures(&lower)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .or_else(|| {
            openjdk_re
                .captures(&lower)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
        })
        .unwrap_or_default();

    version_string = version_string.replace('_', ".").split('-').next().unwrap_or("").to_string();

    if version_string.split('.').count() > 4 {
        version_string = version_string.replace(".0.", ".");
    }

    while version_string.split('.').count() < 4 {
        if version_string.starts_with("1.") {
            version_string.push_str(".0");
        } else {
            version_string = format!("1.{version_string}");
        }
    }

    if version_string.is_empty() {
        return Err(format!(
            "未找到该 Java 的版本号{}",
            if output.len() < 500 {
                format!("\n输出为：\n{output}")
            } else {
                String::new()
            }
        ));
    }

    let parts: Vec<u32> = version_string
        .split('.')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    let mut ver = JavaVersion {
        major: *parts.first().unwrap_or(&0),
        minor: *parts.get(1).unwrap_or(&0),
        build: *parts.get(2).unwrap_or(&0),
        revision: *parts.get(3).unwrap_or(&0),
    };

    if ver.minor == 0 {
        ver = JavaVersion {
            major: 1,
            minor: ver.major,
            build: ver.build,
            revision: ver.revision,
        };
    }

    if ver.minor <= 4 || ver.minor >= 100 {
        return Err(format!("分析详细信息失败，获取的版本为 {}", ver.display_short()));
    }

    Ok(ver)
}
