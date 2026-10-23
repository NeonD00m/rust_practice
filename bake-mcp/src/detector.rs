use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

pub const SUITE_TOOLS: &[&str] = &["todo", "spaghetti", "butternut", "palmtree"];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuiteStatus {
    pub installed_count: usize,
    pub tools: HashMap<String, ToolStatus>,
}

#[derive(Clone, Debug, Default)]
pub struct SuiteDetector;

impl SuiteDetector {
    pub fn new() -> Self {
        Self
    }

    /// Locate the executable path for a given tool name
    pub fn find_binary(&self, name: &str) -> Option<PathBuf> {
        // 1. Check environment variable override (e.g. BAKE_TODO_BIN)
        let env_key = format!("BAKE_{}_BIN", name.to_uppercase());
        if let Ok(val) = std::env::var(&env_key) {
            let p = PathBuf::from(val);
            if p.is_file() {
                return Some(p);
            }
        }

        // 2. Check PATH using the `which` crate
        if let Ok(p) = which::which(name) {
            return Some(p);
        }

        // 3. Check workspace sibling target directories
        if let Ok(curr_dir) = std::env::current_dir() {
            let exe_name = if cfg!(windows) {
                format!("{}.exe", name)
            } else {
                name.to_string()
            };

            let candidates = [
                // Cargo target from workspace root
                curr_dir.join("../target/release").join(&exe_name),
                curr_dir.join("../target/debug").join(&exe_name),
                // Tool-specific local target
                curr_dir.join(format!("../{}/target/release", name)).join(&exe_name),
                curr_dir.join(format!("../{}/target/debug", name)).join(&exe_name),
                // todoCli special directory name
                curr_dir.join("../todoCli/target/release").join(&exe_name),
                curr_dir.join("../todoCli/target/debug").join(&exe_name),
            ];

            for candidate in candidates {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    /// Probe a tool's version by running `<bin> --version`
    pub fn probe_version(&self, path: &PathBuf) -> Option<String> {
        let mut cmd = Command::new(path);
        cmd.arg("--version");

        // Execute with timeout / output inspection
        let output = cmd.output().ok()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let first_line = stdout.lines().next()?.trim();
            if !first_line.is_empty() {
                return Some(first_line.to_string());
            }
        }
        None
    }

    /// Detect the entire Bake productivity suite status
    pub fn detect_suite(&self) -> SuiteStatus {
        let mut tools = HashMap::new();
        let mut installed_count = 0;

        for &tool_name in SUITE_TOOLS {
            if let Some(path) = self.find_binary(tool_name) {
                let version = self.probe_version(&path);
                installed_count += 1;
                tools.insert(
                    tool_name.to_string(),
                    ToolStatus {
                        installed: true,
                        version,
                        path: Some(path.to_string_lossy().to_string()),
                    },
                );
            } else {
                tools.insert(
                    tool_name.to_string(),
                    ToolStatus {
                        installed: false,
                        version: None,
                        path: None,
                    },
                );
            }
        }

        SuiteStatus {
            installed_count,
            tools,
        }
    }

    #[allow(dead_code)]
    pub fn is_installed(&self, name: &str) -> bool {
        self.find_binary(name).is_some()
    }
}
