use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;
use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};
use regex::Regex;
use git2::Repository;

use crate::at_commands::at_tree::TreeNode;
use crate::call_validation::{ChatMessage, ContextFile};
use crate::files_correction::{get_project_dirs, paths_from_anywhere};
use crate::files_in_workspace::detect_vcs_for_a_file_path;
use crate::global_context::GlobalContext;
use crate::git::operations::{get_git_remotes, get_diff_statuses};

const INSTRUCTION_FILE_PATTERNS: &[&str] = &[
    "AGENTS.md",
    "CLAUDE.md",
    "GEMINI.md",
    ".cursorrules",
    "global_rules.md",
    "copilot-instructions.md",
    ".aider.conf.yml",
    "REFACT.md",
];

const RECURSIVE_SEARCH_SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".hg",
    ".svn",
    "target",
    "build",
    "dist",
    "out",
    ".next",
    ".nuxt",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    "venv",
    ".venv",
    "env",
    ".env",
    "vendor",
    ".cargo",
    ".rustup",
    "coverage",
    ".coverage",
    ".tox",
    "eggs",
    "*.egg-info",
    ".gradle",
    ".idea",
    ".vscode",
    ".vs",
];

const RECURSIVE_SEARCH_MAX_DEPTH: usize = 5;

const INSTRUCTION_DIR_PATTERNS: &[(&str, &[&str])] = &[
    (".cursor/rules", &["*.mdc", "*.md"]),
    (".windsurf/rules", &["*.md"]),
    (".github", &["copilot-instructions.md"]),
    (".github/instructions", &["*.instructions.md"]),
    (".claude", &["settings.json", "settings.local.json"]),
    (".refact", &["project_summary.yaml", "instructions.md"]),
    // VSCode - all shareable configs
    (".vscode", &["settings.json", "launch.json", "tasks.json", "extensions.json"]),
    // JetBrains IDEs - shareable configs + workspace.xml (filtered)
    (".idea", &["workspace.xml", "vcs.xml", "misc.xml", "modules.xml", "compiler.xml", "encodings.xml", "jarRepositories.xml"]),
    (".idea/runConfigurations", &["*.xml"]),
    (".idea/codeStyles", &["*.xml"]),
    (".idea/inspectionProfiles", &["*.xml"]),
    // Other IDEs
    (".zed", &["settings.json"]),
    (".fleet", &["settings.json"]),
];

const ENV_MARKERS: &[(&str, &str, &str)] = &[
    // Python
    ("venv", "python_venv", "Python virtual environment"),
    (".venv", "python_venv", "Python virtual environment"),
    ("env", "python_venv", "Python virtual environment (generic name)"),
    (".env", "python_venv", "Python virtual environment (hidden)"),
    ("poetry.lock", "poetry", "Poetry dependency manager"),
    ("pyproject.toml", "python_project", "Python project (PEP 517/518)"),
    ("Pipfile", "pipenv", "Pipenv environment"),
    ("Pipfile.lock", "pipenv", "Pipenv environment"),
    ("requirements.txt", "pip", "Pip requirements"),
    ("setup.py", "python_setuptools", "Python setuptools project"),
    ("conda-meta", "conda", "Conda environment"),
    (".python-version", "pyenv", "Pyenv version file"),
    ("uv.lock", "uv", "UV package manager"),
    // JavaScript/TypeScript/Node
    ("node_modules", "nodejs", "Node.js modules"),
    ("package.json", "nodejs", "Node.js project"),
    ("package-lock.json", "npm", "NPM package manager"),
    ("yarn.lock", "yarn", "Yarn package manager"),
    ("pnpm-lock.yaml", "pnpm", "PNPM package manager"),
    ("bun.lockb", "bun", "Bun runtime"),
    (".nvmrc", "nvm", "Node Version Manager"),
    (".node-version", "nodenv", "Node version file"),
    ("deno.json", "deno", "Deno runtime"),
    ("deno.lock", "deno", "Deno runtime"),
    // Rust
    ("Cargo.toml", "cargo", "Rust/Cargo project"),
    ("Cargo.lock", "cargo", "Rust/Cargo project"),
    ("rust-toolchain.toml", "rustup", "Rust toolchain"),
    ("rust-toolchain", "rustup", "Rust toolchain"),
    // Go
    ("go.mod", "go_modules", "Go modules"),
    ("go.sum", "go_modules", "Go modules"),
    ("Gopkg.toml", "go_dep", "Go dep (legacy)"),
    // Java/JVM
    ("pom.xml", "maven", "Maven project"),
    ("build.gradle", "gradle", "Gradle project"),
    ("build.gradle.kts", "gradle_kotlin", "Gradle Kotlin DSL"),
    ("gradlew", "gradle", "Gradle wrapper"),
    (".mvn", "maven", "Maven wrapper"),
    ("build.sbt", "sbt", "SBT (Scala) project"),
    // Ruby
    ("Gemfile", "bundler", "Ruby Bundler"),
    ("Gemfile.lock", "bundler", "Ruby Bundler"),
    (".ruby-version", "rbenv", "Ruby version file"),
    (".rvmrc", "rvm", "RVM configuration"),
    // PHP
    ("composer.json", "composer", "PHP Composer"),
    ("composer.lock", "composer", "PHP Composer"),
    // .NET
    ("*.csproj", "dotnet", ".NET project"),
    ("*.fsproj", "dotnet", "F# project"),
    ("*.sln", "dotnet", ".NET solution"),
    ("nuget.config", "nuget", "NuGet configuration"),
    ("global.json", "dotnet", ".NET SDK version"),
    // Elixir
    ("mix.exs", "mix", "Elixir Mix project"),
    ("mix.lock", "mix", "Elixir Mix project"),
    // Docker/Containers
    ("Dockerfile", "docker", "Docker container"),
    ("docker-compose.yml", "docker_compose", "Docker Compose"),
    ("docker-compose.yaml", "docker_compose", "Docker Compose"),
    ("compose.yml", "docker_compose", "Docker Compose"),
    ("compose.yaml", "docker_compose", "Docker Compose"),
    (".devcontainer", "devcontainer", "Dev Container"),
    ("devcontainer.json", "devcontainer", "Dev Container"),
    // Build/Task runners
    ("Makefile", "make", "Make build system"),
    ("CMakeLists.txt", "cmake", "CMake build system"),
    ("justfile", "just", "Just command runner"),
    ("Taskfile.yml", "task", "Task runner"),
    // CI/CD
    (".github/workflows", "github_actions", "GitHub Actions"),
    (".gitlab-ci.yml", "gitlab_ci", "GitLab CI"),
    ("Jenkinsfile", "jenkins", "Jenkins pipeline"),
    (".circleci", "circleci", "CircleCI"),
    (".travis.yml", "travis", "Travis CI"),
];

const CONFIG_FILES: &[&str] = &[
    ".editorconfig",
    ".prettierrc",
    ".prettierrc.json",
    ".prettierrc.yml",
    ".eslintrc",
    ".eslintrc.json",
    ".eslintrc.yml",
    "eslint.config.js",
    "eslint.config.mjs",
    ".stylelintrc",
    "tsconfig.json",
    "jsconfig.json",
    "biome.json",
    ".pre-commit-config.yaml",
    ".commitlintrc",
    ".husky",
    ".lintstagedrc",
    "jest.config.js",
    "jest.config.ts",
    "vitest.config.ts",
    "pytest.ini",
    "setup.cfg",
    "tox.ini",
    ".coveragerc",
    "karma.conf.js",
    "cypress.config.js",
    "playwright.config.ts",
    "mkdocs.yml",
    "docusaurus.config.js",
    "vuepress.config.js",
    "book.toml",
    "webpack.config.js",
    "vite.config.ts",
    "vite.config.js",
    "rollup.config.js",
    "esbuild.config.js",
    "turbo.json",
    "nx.json",
    "lerna.json",
    ".env.example",
    ".env.template",
    ".env.local.example",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub username: String,
    pub hostname: String,
    pub home_dir: String,
    pub current_dir: String,
    pub datetime_local: String,
    pub datetime_utc: String,
    pub timezone: String,
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedEnvironment {
    pub env_type: String,
    pub description: String,
    pub path: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionFile {
    pub file_name: String,
    pub file_path: String,
    pub source_tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub file_name: String,
    pub file_path: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub vcs_type: String,
    pub repo_root: String,
    pub current_branch: Option<String>,
    pub branches: Vec<String>,
    pub remotes: Vec<(String, String)>,
    pub staged_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub is_dirty: bool,
}

impl GitInfo {
    pub fn to_prompt_string(&self) -> String {
        let mut lines = Vec::new();

        if let Some(ref branch) = self.current_branch {
            lines.push(format!("**Current Branch**: `{}`", branch));
        }

        if !self.branches.is_empty() {
            let other_branches: Vec<_> = self.branches.iter()
                .filter(|b| Some(*b) != self.current_branch.as_ref())
                .take(10)
                .collect();
            if !other_branches.is_empty() {
                let branch_list = other_branches.iter()
                    .map(|b| format!("`{}`", b))
                    .collect::<Vec<_>>()
                    .join(", ");
                let suffix = if self.branches.len() > 11 {
                    format!(" (+{} more)", self.branches.len() - 11)
                } else {
                    String::new()
                };
                lines.push(format!("**Other Branches**: {}{}", branch_list, suffix));
            }
        }

        if !self.remotes.is_empty() {
            let remote_list = self.remotes.iter()
                .map(|(name, url)| format!("`{}` → {}", name, url))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("**Remotes**: {}", remote_list));
        }

        if !self.staged_files.is_empty() {
            lines.push(format!("**Staged** ({} files): {}",
                self.staged_files.len(),
                format_file_list(&self.staged_files, 5)
            ));
        }

        if !self.modified_files.is_empty() {
            lines.push(format!("**Modified** ({} files): {}",
                self.modified_files.len(),
                format_file_list(&self.modified_files, 5)
            ));
        }

        if !self.untracked_files.is_empty() {
            lines.push(format!("**Untracked** ({} files): {}",
                self.untracked_files.len(),
                format_file_list(&self.untracked_files, 5)
            ));
        }

        if self.staged_files.is_empty() && self.modified_files.is_empty() && self.untracked_files.is_empty() {
            lines.push("**Status**: Clean working directory".to_string());
        }

        lines.join("\n")
    }
}

fn format_file_list(files: &[String], max_show: usize) -> String {
    let shown: Vec<_> = files.iter().take(max_show).map(|f| format!("`{}`", f)).collect();
    let remaining = files.len().saturating_sub(max_show);
    if remaining > 0 {
        format!("{} (+{} more)", shown.join(", "), remaining)
    } else {
        shown.join(", ")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub system_info: SystemInfo,
    pub detected_environments: Vec<DetectedEnvironment>,
    pub instruction_files: Vec<InstructionFile>,
    pub project_configs: Vec<ProjectConfig>,
    pub project_tree: Option<String>,
    pub environment_instructions: String,
    pub git_info: Vec<GitInfo>,
}

impl SystemInfo {
    pub fn gather() -> Self {
        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();
        let os_version = Self::get_os_version();
        let username = Self::get_username();
        let hostname = Self::get_hostname();
        let home_dir = home::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let current_dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let now_local = Local::now();
        let now_utc = Utc::now();

        SystemInfo {
            os,
            os_version,
            arch,
            username,
            hostname,
            home_dir,
            current_dir,
            datetime_local: now_local.format("%Y-%m-%d %H:%M:%S").to_string(),
            datetime_utc: now_utc.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            timezone: now_local.format("%Z").to_string(),
            shell: std::env::var("SHELL").ok().or_else(|| std::env::var("COMSPEC").ok()),
        }
    }

    fn get_os_version() -> String {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        return line
                            .trim_start_matches("PRETTY_NAME=")
                            .trim_matches('"')
                            .to_string();
                    }
                }
            }
            "Linux".to_string()
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("sw_vers")
                .arg("-productVersion")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|v| format!("macOS {}", v.trim()))
                .unwrap_or_else(|| "macOS".to_string())
        }
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/c", "ver"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|v| v.trim().to_string())
                .unwrap_or_else(|| "Windows".to_string())
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            "Unknown".to_string()
        }
    }

    fn get_username() -> String {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    }

    fn get_hostname() -> String {
        hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn to_prompt_string(&self) -> String {
        let mut lines = vec![
            "## System Information".to_string(),
            format!("- **OS**: {} ({})", self.os_version, self.arch),
            format!("- **User**: {}@{}", self.username, self.hostname),
            format!("- **DateTime**: {} ({})", self.datetime_local, self.timezone),
        ];
        if let Some(shell) = &self.shell {
            lines.push(format!("- **Shell**: {}", shell));
        }
        lines.join("\n")
    }
}

pub async fn detect_environments(project_dirs: &[PathBuf]) -> Vec<DetectedEnvironment> {
    let mut environments = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for project_dir in project_dirs {
        for (marker, env_type, description) in ENV_MARKERS {
            let marker_path = project_dir.join(marker);
            let exists = if marker.contains('*') {
                if let Ok(entries) = std::fs::read_dir(project_dir) {
                    entries.filter_map(|e| e.ok()).any(|entry| {
                        let name = entry.file_name().to_string_lossy().to_string();
                        glob_match(marker, &name)
                    })
                } else {
                    false
                }
            } else {
                marker_path.exists()
            };

            if exists {
                let path_str = marker_path.to_string_lossy().to_string();
                let key = format!("{}:{}", env_type, project_dir.display());

                if !seen.contains(&key) {
                    seen.insert(key);
                    let is_active = check_env_active(env_type, &marker_path);
                    environments.push(DetectedEnvironment {
                        env_type: env_type.to_string(),
                        description: description.to_string(),
                        path: path_str,
                        is_active,
                    });
                }
            }
        }
    }

    environments.sort_by(|a, b| a.env_type.cmp(&b.env_type));
    environments
}

fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return text.ends_with(&format!(".{}", suffix));
    }
    if let Some(prefix) = pattern.strip_suffix("*") {
        return text.starts_with(prefix);
    }
    pattern == text
}

fn check_env_active(env_type: &str, marker_path: &Path) -> bool {
    match env_type {
        "python_venv" => {
            if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
                let venv_path = PathBuf::from(&venv);
                if let Some(parent) = marker_path.parent() {
                    return venv_path == parent.join(marker_path.file_name().unwrap_or_default())
                        || venv_path == *marker_path;
                }
            }
            false
        }
        "conda" => std::env::var("CONDA_DEFAULT_ENV").is_ok(),
        _ => false,
    }
}

fn extract_workspace_xml_important_parts(content: &str) -> Option<String> {
    let mut configs = Vec::new();

    let pattern = r#"<component\s+name\s*=\s*"RunManager"[^>]*>[\s\S]*?</component>"#;
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return None,
    };

    if let Some(run_manager_match) = re.find(content) {
        let run_manager_xml = run_manager_match.as_str();

        let selected = Regex::new(r#"selected="([^"]*)""#).ok()
            .and_then(|r| r.captures(run_manager_xml))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        let config_pattern = r#"<configuration[^>]*>[\s\S]*?</configuration>"#;
        if let Ok(config_re) = Regex::new(config_pattern) {
            for config_match in config_re.find_iter(run_manager_xml) {
                let config = config_match.as_str();

                if config.contains(r#"default="true""#) || config.contains(r#"temporary="true""#) {
                    continue;
                }

                if let Some(parsed) = parse_run_configuration(config) {
                    configs.push(parsed);
                }
            }
        }

        if configs.is_empty() {
            return None;
        }

        let mut result = String::from("# IDE Run Configurations\n");
        if let Some(sel) = selected {
            result.push_str(&format!("selected: {}\n", sel));
        }
        result.push_str("configurations:\n");

        for cfg in configs {
            result.push_str(&format!("  - name: {}\n", cfg.name));
            result.push_str(&format!("    type: {}\n", cfg.config_type));
            if !cfg.command.is_empty() {
                result.push_str(&format!("    command: {}\n", cfg.command));
            }
            if !cfg.workdir.is_empty() {
                result.push_str(&format!("    workdir: {}\n", cfg.workdir));
            }
            if !cfg.envs.is_empty() {
                result.push_str("    env:\n");
                for (k, v) in &cfg.envs {
                    result.push_str(&format!("      {}: {}\n", k, v));
                }
            }
            if !cfg.extra.is_empty() {
                for (k, v) in &cfg.extra {
                    result.push_str(&format!("    {}: {}\n", k, v));
                }
            }
        }

        return Some(result);
    }

    None
}

struct RunConfig {
    name: String,
    config_type: String,
    command: String,
    workdir: String,
    envs: Vec<(String, String)>,
    extra: Vec<(String, String)>,
}

fn parse_run_configuration(config_xml: &str) -> Option<RunConfig> {
    let name = extract_xml_attr(config_xml, "name")?;
    let config_type = extract_xml_attr(config_xml, "type").unwrap_or_default();

    let mut command = String::new();
    let mut workdir = String::new();
    let mut envs = Vec::new();
    let mut extra = Vec::new();

    if let Some(cmd) = extract_option_value(config_xml, "command") {
        command = cmd;
    }

    if let Some(wd) = extract_option_value(config_xml, "workingDirectory") {
        workdir = wd;
    } else if let Some(wd) = extract_option_value(config_xml, "WORKING_DIRECTORY") {
        workdir = wd;
    }

    if let Ok(env_re) = Regex::new(r#"<env\s+name="([^"]*)"\s+value="([^"]*)"\s*/>"#) {
        for cap in env_re.captures_iter(config_xml) {
            if let (Some(k), Some(v)) = (cap.get(1), cap.get(2)) {
                envs.push((k.as_str().to_string(), v.as_str().to_string()));
            }
        }
    }

    if let Ok(envs_map_re) = Regex::new(r#"<envs>[\s\S]*?</envs>"#) {
        if let Some(envs_match) = envs_map_re.find(config_xml) {
            if let Ok(entry_re) = Regex::new(r#"<env\s+name="([^"]*)"\s+value="([^"]*)""#) {
                for cap in entry_re.captures_iter(envs_match.as_str()) {
                    if let (Some(k), Some(v)) = (cap.get(1), cap.get(2)) {
                        envs.push((k.as_str().to_string(), v.as_str().to_string()));
                    }
                }
            }
        }
    }

    if config_type.contains("Cargo") {
        if let Some(channel) = extract_option_value(config_xml, "channel") {
            if channel != "DEFAULT" {
                extra.push(("channel".to_string(), channel));
            }
        }
        if let Some(bt) = extract_option_value(config_xml, "backtrace") {
            if bt != "SHORT" {
                extra.push(("backtrace".to_string(), bt));
            }
        }
    }

    if config_type.contains("Python") || config_type.contains("Django") {
        if let Some(script) = extract_option_value(config_xml, "SCRIPT_NAME") {
            command = script;
        }
        if let Some(params) = extract_option_value(config_xml, "PARAMETERS") {
            if !params.is_empty() {
                command = format!("{} {}", command, params).trim().to_string();
            }
        }
    }

    if config_type.contains("NodeJS") || config_type.contains("npm") {
        if let Some(script) = extract_option_value(config_xml, "node-parameters") {
            command = script;
        }
    }

    Some(RunConfig {
        name,
        config_type,
        command,
        workdir,
        envs,
        extra,
    })
}

fn extract_xml_attr(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"{}="([^"]*)""#, regex::escape(attr));
    Regex::new(&pattern).ok()
        .and_then(|re| re.captures(xml))
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

fn extract_option_value(xml: &str, option_name: &str) -> Option<String> {
    let pattern = format!(r#"<option\s+name="{}"\s+value="([^"]*)""#, regex::escape(option_name));
    Regex::new(&pattern).ok()
        .and_then(|re| re.captures(xml))
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

fn should_skip_dir(dir_name: &str) -> bool {
    for skip_pattern in RECURSIVE_SEARCH_SKIP_DIRS {
        if skip_pattern.starts_with("*.") {
            if let Some(suffix) = skip_pattern.strip_prefix("*.") {
                if dir_name.ends_with(suffix) {
                    return true;
                }
            }
        } else if dir_name == *skip_pattern {
            return true;
        }
    }
    false
}

fn find_instruction_files_recursive(
    dir: &Path,
    depth: usize,
    seen_paths: &mut std::collections::HashSet<String>,
    files: &mut Vec<InstructionFile>,
) {
    if depth > RECURSIVE_SEARCH_MAX_DEPTH {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        let entry_name = entry.file_name().to_string_lossy().to_string();

        if entry_path.is_dir() {
            if !should_skip_dir(&entry_name) {
                find_instruction_files_recursive(&entry_path, depth + 1, seen_paths, files);
            }
        } else if entry_path.is_file() {
            for pattern in INSTRUCTION_FILE_PATTERNS {
                if entry_name == *pattern {
                    let path_str = entry_path.to_string_lossy().to_string();
                    if !seen_paths.contains(&path_str) {
                        seen_paths.insert(path_str.clone());
                        tracing::info!("Found instruction file (recursive): {}", path_str);
                        files.push(InstructionFile {
                            file_name: entry_name.clone(),
                            file_path: path_str,
                            source_tool: determine_tool_source(pattern),
                            processed_content: None,
                        });
                    }
                    break;
                }
            }
        }
    }
}

pub async fn find_instruction_files(project_dirs: &[PathBuf]) -> Vec<InstructionFile> {
    let mut files = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for project_dir in project_dirs {
        find_instruction_files_recursive(project_dir, 0, &mut seen_paths, &mut files);

        for (dir_pattern, file_patterns) in INSTRUCTION_DIR_PATTERNS {
            let dir_path = project_dir.join(dir_pattern);
            if dir_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&dir_path) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_name = entry.file_name().to_string_lossy().to_string();
                        for file_pattern in *file_patterns {
                            if glob_match(file_pattern, &entry_name) {
                                let file_path = entry.path();
                                let path_str = file_path.to_string_lossy().to_string();
                                if !seen_paths.contains(&path_str) && file_path.is_file() {
                                    seen_paths.insert(path_str.clone());

                                    let processed_content = if entry_name == "workspace.xml" {
                                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                                            extract_workspace_xml_important_parts(&content)
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };

                                    if entry_name == "workspace.xml" && processed_content.is_none() {
                                        continue;
                                    }

                                    tracing::info!("Found instruction file (dir pattern {}): {}", dir_pattern, path_str);
                                    files.push(InstructionFile {
                                        file_name: entry_name.clone(),
                                        file_path: path_str,
                                        source_tool: determine_tool_source(dir_pattern),
                                        processed_content,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut current = project_dir.clone();
        for _ in 0..3 {
            if let Some(parent) = current.parent() {
                for pattern in &["AGENTS.md", "CLAUDE.md", "REFACT.md"] {
                    let file_path = parent.join(pattern);
                    if file_path.exists() && file_path.is_file() {
                        let path_str = file_path.to_string_lossy().to_string();
                        if !seen_paths.contains(&path_str) {
                            seen_paths.insert(path_str.clone());
                            tracing::info!("Found instruction file (parent dir): {}", path_str);
                            files.push(InstructionFile {
                                file_name: pattern.to_string(),
                                file_path: path_str,
                                source_tool: determine_tool_source(pattern),
                                processed_content: None,
                            });
                        }
                    }
                }
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
    }

    tracing::info!(
        "Instruction files search complete: found {} files total",
        files.len()
    );

    files
}

fn determine_tool_source(pattern: &str) -> String {
    match pattern.to_lowercase().as_str() {
        "agents.md" => "universal".to_string(),
        "claude.md" | ".claude" => "claude_code".to_string(),
        "gemini.md" => "gemini".to_string(),
        ".cursorrules" | ".cursor/rules" => "cursor".to_string(),
        "global_rules.md" | ".windsurf/rules" => "windsurf".to_string(),
        "copilot-instructions.md" | ".github" | ".github/instructions" => "github_copilot".to_string(),
        ".aider.conf.yml" => "aider".to_string(),
        "refact.md" | ".refact" => "refact".to_string(),
        _ => "unknown".to_string(),
    }
}

pub async fn find_project_configs(project_dirs: &[PathBuf]) -> Vec<ProjectConfig> {
    let mut configs = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for project_dir in project_dirs {
        for config_file in CONFIG_FILES {
            let file_path = project_dir.join(config_file);
            if file_path.exists() {
                let path_str = file_path.to_string_lossy().to_string();
                if !seen_paths.contains(&path_str) {
                    seen_paths.insert(path_str.clone());
                    configs.push(ProjectConfig {
                        file_name: config_file.to_string(),
                        file_path: path_str,
                        category: categorize_config(config_file),
                    });
                }
            }
        }
    }

    configs
}

fn categorize_config(file_name: &str) -> String {
    let lower = file_name.to_lowercase();
    if lower.contains("eslint")
        || lower.contains("prettier")
        || lower.contains("stylelint")
        || lower.contains("editorconfig")
        || lower.contains("biome")
    {
        "code_style".to_string()
    } else if lower.contains("jest")
        || lower.contains("vitest")
        || lower.contains("pytest")
        || lower.contains("coverage")
        || lower.contains("karma")
        || lower.contains("cypress")
        || lower.contains("playwright")
    {
        "testing".to_string()
    } else if lower.contains("webpack")
        || lower.contains("vite")
        || lower.contains("rollup")
        || lower.contains("esbuild")
        || lower.contains("turbo")
    {
        "build".to_string()
    } else if lower.contains("tsconfig") || lower.contains("jsconfig") {
        "typescript".to_string()
    } else if lower.contains("commit") || lower.contains("husky") || lower.contains("pre-commit") {
        "git_hooks".to_string()
    } else if lower.contains("mkdocs") || lower.contains("docusaurus") || lower.contains("book.toml") {
        "documentation".to_string()
    } else if lower.contains("env") {
        "environment".to_string()
    } else {
        "other".to_string()
    }
}

pub async fn gather_git_info(project_dirs: &[PathBuf]) -> Vec<GitInfo> {
    let mut git_infos = Vec::new();
    let mut seen_repos = std::collections::HashSet::new();

    for project_dir in project_dirs {
        if let Some((vcs_root, vcs_type)) = detect_vcs_for_a_file_path(project_dir).await {
            let root_str = vcs_root.to_string_lossy().to_string();
            if seen_repos.contains(&root_str) {
                continue;
            }
            seen_repos.insert(root_str.clone());

            if vcs_type != "git" {
                git_infos.push(GitInfo {
                    vcs_type: vcs_type.to_string(),
                    repo_root: root_str,
                    current_branch: None,
                    branches: vec![],
                    remotes: vec![],
                    staged_files: vec![],
                    modified_files: vec![],
                    untracked_files: vec![],
                    is_dirty: false,
                });
                continue;
            }

            match Repository::open(&vcs_root) {
                Ok(repo) => {
                    let current_branch = repo.head().ok()
                        .and_then(|h| h.shorthand().map(String::from));

                    let branches = repo.branches(Some(git2::BranchType::Local))
                        .map(|branches| {
                            branches
                                .filter_map(|b| b.ok())
                                .filter_map(|(branch, _)| branch.name().ok().flatten().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    let remotes = get_git_remotes(&vcs_root).unwrap_or_default();

                    let (staged, unstaged) = get_diff_statuses(
                        git2::StatusShow::IndexAndWorkdir,
                        &repo,
                        false
                    ).unwrap_or_default();

                    let staged_files: Vec<String> = staged.iter()
                        .map(|f| f.relative_path.to_string_lossy().to_string())
                        .collect();

                    let mut modified_files = Vec::new();
                    let mut untracked_files = Vec::new();

                    for file in &unstaged {
                        let path_str = file.relative_path.to_string_lossy().to_string();
                        match file.status {
                            crate::git::FileChangeStatus::ADDED => untracked_files.push(path_str),
                            _ => modified_files.push(path_str),
                        }
                    }

                    let is_dirty = !staged_files.is_empty() || !modified_files.is_empty();

                    tracing::info!(
                        "Git info for {}: branch={:?}, {} staged, {} modified, {} untracked",
                        root_str,
                        current_branch,
                        staged_files.len(),
                        modified_files.len(),
                        untracked_files.len()
                    );

                    git_infos.push(GitInfo {
                        vcs_type: "git".to_string(),
                        repo_root: root_str,
                        current_branch,
                        branches,
                        remotes,
                        staged_files,
                        modified_files,
                        untracked_files,
                        is_dirty,
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to open git repository at {}: {}", root_str, e);
                }
            }
        }
    }

    git_infos
}

pub fn generate_git_info_prompt(git_infos: &[GitInfo]) -> String {
    if git_infos.is_empty() {
        return String::new();
    }

    let mut lines = vec!["## Version Control".to_string()];

    for info in git_infos {
        if git_infos.len() > 1 {
            lines.push(format!("\n### {} ({})", info.repo_root, info.vcs_type));
        }
        lines.push(info.to_prompt_string());
    }

    lines.join("\n")
}

pub fn generate_environment_instructions(environments: &[DetectedEnvironment]) -> String {
    let mut instructions = Vec::new();

    instructions.push("## Local Environment Usage Guidelines".to_string());
    instructions.push(String::new());
    instructions.push("**IMPORTANT**: Always prefer local/project-specific environments over system-wide installations.".to_string());
    instructions.push(String::new());
    instructions.push("## Internal Context Files Notice\nThe instruction and configuration files (ie .idea) shared at the start of this conversation (AGENTS.md, IDE configs, .cursorrules, etc.) are internal context provided automatically to help you understand the project structure, IDE setup, and coding guidelines.\n**Do NOT explicitly mention, reference, or acknowledge these files to the user**!!!".to_string());

    let mut python_envs: Vec<&DetectedEnvironment> = Vec::new();
    let mut node_envs: Vec<&DetectedEnvironment> = Vec::new();
    let mut other_envs: Vec<&DetectedEnvironment> = Vec::new();

    for env in environments {
        match env.env_type.as_str() {
            "python_venv" | "poetry" | "pipenv" | "conda" | "uv" | "pyenv" => {
                python_envs.push(env);
            }
            "nodejs" | "npm" | "yarn" | "pnpm" | "bun" | "nvm" | "nodenv" | "deno" => {
                node_envs.push(env);
            }
            _ => {
                other_envs.push(env);
            }
        }
    }

    if !python_envs.is_empty() {
        instructions.push("### Python".to_string());
        for env in &python_envs {
            let active_marker = if env.is_active { " ✓ (active)" } else { "" };
            instructions.push(format!("- **{}**: `{}`{}", env.description, env.path, active_marker));
        }

        let has_venv = python_envs.iter().any(|e| e.env_type == "python_venv");
        let has_poetry = python_envs.iter().any(|e| e.env_type == "poetry");
        let has_uv = python_envs.iter().any(|e| e.env_type == "uv");

        instructions.push(String::new());
        if has_uv {
            instructions.push("**Preferred**: Use `uv` for Python package management:".to_string());
            instructions.push("```bash".to_string());
            instructions.push("uv pip install <package>".to_string());
            instructions.push("uv run python <script.py>".to_string());
            instructions.push("```".to_string());
        } else if has_poetry {
            instructions.push("**Preferred**: Use `poetry` for Python package management:".to_string());
            instructions.push("```bash".to_string());
            instructions.push("poetry install".to_string());
            instructions.push("poetry run python <script.py>".to_string());
            instructions.push("```".to_string());
        } else if has_venv {
            if let Some(venv) = python_envs.iter().find(|e| e.env_type == "python_venv") {
                instructions.push("**Preferred**: Use the virtual environment directly (no activation needed):".to_string());
                instructions.push("```bash".to_string());
                if cfg!(windows) {
                    instructions.push(format!("{}/Scripts/python.exe <script.py>", venv.path));
                    instructions.push(format!("{}/Scripts/pip.exe install <package>", venv.path));
                } else {
                    instructions.push(format!("{}/bin/python <script.py>", venv.path));
                    instructions.push(format!("{}/bin/pip install <package>", venv.path));
                }
                instructions.push("```".to_string());
            }
        }
        instructions.push(String::new());
    }

    if !node_envs.is_empty() {
        instructions.push("### JavaScript/Node.js".to_string());
        for env in &node_envs {
            instructions.push(format!("- **{}**: `{}`", env.description, env.path));
        }

        let has_pnpm = node_envs.iter().any(|e| e.env_type == "pnpm");
        let has_yarn = node_envs.iter().any(|e| e.env_type == "yarn");
        let has_bun = node_envs.iter().any(|e| e.env_type == "bun");

        instructions.push(String::new());
        if has_bun {
            instructions.push("**Preferred**: Use `bun` as the runtime/package manager:".to_string());
            instructions.push("```bash".to_string());
            instructions.push("bun install".to_string());
            instructions.push("bun run <script>".to_string());
            instructions.push("```".to_string());
        } else if has_pnpm {
            instructions.push("**Preferred**: Use `pnpm` as the package manager:".to_string());
            instructions.push("```bash".to_string());
            instructions.push("pnpm install".to_string());
            instructions.push("pnpm run <script>".to_string());
            instructions.push("```".to_string());
        } else if has_yarn {
            instructions.push("**Preferred**: Use `yarn` as the package manager:".to_string());
            instructions.push("```bash".to_string());
            instructions.push("yarn install".to_string());
            instructions.push("yarn <script>".to_string());
            instructions.push("```".to_string());
        } else {
            instructions.push("**Package manager**: npm".to_string());
            instructions.push("```bash".to_string());
            instructions.push("npm install".to_string());
            instructions.push("npm run <script>".to_string());
            instructions.push("```".to_string());
        }
        instructions.push(String::new());
    }

    if !other_envs.is_empty() {
        instructions.push("### Other Environments".to_string());
        for env in &other_envs {
            if !matches!(
                env.env_type.as_str(),
                "pip" | "python_project" | "python_setuptools" | "nodejs"
            ) {
                instructions.push(format!("- **{}**: `{}`", env.description, env.path));
            }
        }
        instructions.push(String::new());
    }

    instructions.join("\n")
}

pub async fn generate_compact_project_tree(
    gcx: Arc<ARwLock<GlobalContext>>,
    max_depth: usize,
) -> Result<String, String> {
    let paths = paths_from_anywhere(gcx.clone()).await;
    let project_dirs = get_project_dirs(gcx.clone()).await;

    let mut result = String::new();

    for project_dir in &project_dirs {
        let project_name = project_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project_dir.to_string_lossy().to_string());

        let relative_paths: Vec<PathBuf> = paths
            .iter()
            .filter(|path| path.starts_with(project_dir))
            .filter_map(|path| path.strip_prefix(project_dir).ok())
            .map(|p| p.to_path_buf())
            .collect();

        if relative_paths.is_empty() {
            continue;
        }

        let tree = TreeNode::build(&relative_paths);
        let tree_str = print_compact_tree(&tree, &project_name, max_depth);
        result.push_str(&tree_str);
    }

    Ok(result)
}

fn print_compact_tree(tree: &TreeNode, project_name: &str, max_depth: usize) -> String {
    fn traverse(
        node: &TreeNode,
        name: &str,
        depth: usize,
        max_depth: usize,
        output: &mut String,
    ) {
        if depth > max_depth {
            return;
        }

        let indent = "  ".repeat(depth);

        if node.is_dir() {
            output.push_str(&format!("{}{}/\n", indent, name));

            let mut entries: Vec<_> = node.children.iter().collect();
            entries.sort_by(|a, b| {
                let a_is_dir = a.1.is_dir();
                let b_is_dir = b.1.is_dir();
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.0.cmp(b.0),
                }
            });

            for (child_name, child) in entries {
                traverse(child, child_name, depth + 1, max_depth, output);
            }
        } else {
            output.push_str(&format!("{}{}\n", indent, name));
        }
    }

    let mut result = String::new();
    result.push_str(&format!("{}/\n", project_name));

    let mut entries: Vec<_> = tree.children.iter().collect();
    entries.sort_by(|a, b| {
        let a_is_dir = a.1.is_dir();
        let b_is_dir = b.1.is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.0.cmp(b.0),
        }
    });

    for (name, node) in entries {
        traverse(node, name, 1, max_depth, &mut result);
    }

    result
}

pub async fn gather_system_context(
    gcx: Arc<ARwLock<GlobalContext>>,
    include_tree: bool,
    tree_max_depth: usize,
) -> Result<SystemContext, String> {
    let system_info = SystemInfo::gather();
    let project_dirs = get_project_dirs(gcx.clone()).await;

    let detected_environments = detect_environments(&project_dirs).await;
    let instruction_files = find_instruction_files(&project_dirs).await;
    let project_configs = find_project_configs(&project_dirs).await;
    let git_info = gather_git_info(&project_dirs).await;

    let project_tree = if include_tree {
        generate_compact_project_tree(gcx.clone(), tree_max_depth).await.ok()
    } else {
        None
    };

    let environment_instructions = generate_environment_instructions(&detected_environments);

    Ok(SystemContext {
        system_info,
        detected_environments,
        instruction_files,
        project_configs,
        project_tree,
        environment_instructions,
        git_info,
    })
}

const MAX_FILE_SIZE: usize = 10_000;
const MAX_INCLUDED_FILES: usize = 10;

pub async fn create_instruction_files_message(
    instruction_files: &[InstructionFile],
) -> Result<ChatMessage, String> {
    let mut context_files = Vec::new();
    let mut paths_only: Vec<String> = Vec::new();

    for (idx, instr_file) in instruction_files.iter().enumerate() {
        if idx >= MAX_INCLUDED_FILES {
            paths_only.push(instr_file.file_path.clone());
            continue;
        }

        let content = if let Some(ref processed) = instr_file.processed_content {
            processed.clone()
        } else {
            match tokio::fs::read_to_string(&instr_file.file_path).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Failed to read instruction file {}: {}", instr_file.file_path, e);
                    continue;
                }
            }
        };

        let content_len = content.len();
        let (final_content, was_truncated) = if content_len > MAX_FILE_SIZE {
            let truncated = content.chars().take(MAX_FILE_SIZE).collect::<String>();
            (truncated, true)
        } else {
            (content, false)
        };

        let mut display_name = instr_file.file_path.clone();
        if instr_file.processed_content.is_some() {
            display_name = format!("{} (filtered)", display_name);
        }
        if was_truncated {
            display_name = format!("{} (truncated)", display_name);
            tracing::info!("Truncated instruction file {} from {} to {} chars",
                instr_file.file_path, content_len, MAX_FILE_SIZE);
        }

        context_files.push(ContextFile {
            file_name: display_name,
            file_content: final_content.clone(),
            line1: 1,
            line2: final_content.lines().count().max(1),
            symbols: vec![],
            gradient_type: 0,
            usefulness: 100.0,
            skip_pp: false,
        });
    }

    if !paths_only.is_empty() {
        let paths_content = format!(
            "Additional instruction files (paths only, limit of {} full files reached):\n{}",
            MAX_INCLUDED_FILES,
            paths_only.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n")
        );
        context_files.push(ContextFile {
            file_name: "(additional files - paths only)".to_string(),
            file_content: paths_content.clone(),
            line1: 1,
            line2: paths_content.lines().count().max(1),
            symbols: vec![],
            gradient_type: 0,
            usefulness: 50.0,
            skip_pp: false,
        });
        tracing::info!("Listed {} additional instruction files as paths only", paths_only.len());
    }

    if context_files.is_empty() {
        return Err("No instruction files found or readable".to_string());
    }

    tracing::info!(
        "Created instruction files message: {} full files, {} paths only",
        context_files.len().saturating_sub(if paths_only.is_empty() { 0 } else { 1 }),
        paths_only.len()
    );

    let context_files_json = serde_json::to_string(&context_files)
        .map_err(|e| format!("Failed to serialize context files: {}", e))?;

    Ok(ChatMessage::new("cd_instruction".to_string(), context_files_json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_gather() {
        let info = SystemInfo::gather();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(!info.username.is_empty());
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.md", "README.md"));
        assert!(glob_match("*.md", "AGENTS.md"));
        assert!(!glob_match("*.md", "config.json"));
        assert!(glob_match("*.mdc", "rules.mdc"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn test_determine_tool_source() {
        assert_eq!(determine_tool_source("AGENTS.md"), "universal");
        assert_eq!(determine_tool_source("CLAUDE.md"), "claude_code");
        assert_eq!(determine_tool_source(".cursorrules"), "cursor");
    }

    #[test]
    fn test_categorize_config() {
        assert_eq!(categorize_config(".eslintrc"), "code_style");
        assert_eq!(categorize_config("jest.config.js"), "testing");
        assert_eq!(categorize_config("webpack.config.js"), "build");
        assert_eq!(categorize_config("tsconfig.json"), "typescript");
    }

    #[test]
    fn test_extract_workspace_xml() {
        let workspace_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project version="4">
  <component name="ChangeListManager">
    <list default="true" id="123" name="Changes" />
  </component>
  <component name="RunManager" selected="Application.Main">
    <configuration default="true" type="Application" factoryName="Application">
      <option name="MAIN_CLASS_NAME" value="" />
    </configuration>
    <configuration name="Main" type="Application" factoryName="Application">
      <option name="MAIN_CLASS_NAME" value="com.example.Main" />
      <option name="workingDirectory" value="$PROJECT_DIR$" />
    </configuration>
    <configuration name="Test" type="Application" temporary="true">
      <option name="MAIN_CLASS_NAME" value="com.example.Test" />
    </configuration>
  </component>
  <component name="ProjectId" id="abc123" />
</project>"#;

        let extracted = extract_workspace_xml_important_parts(workspace_xml);
        assert!(extracted.is_some());
        let result = extracted.unwrap();
        assert!(result.contains("configurations:"));
        assert!(result.contains("name: Main"));
        assert!(result.contains("type: Application"));
        assert!(result.contains("selected: Application.Main"));
        assert!(!result.contains("ChangeListManager"));
        assert!(!result.contains("ProjectId"));
        assert!(!result.contains("Test")); // temporary config should be excluded
    }
}
