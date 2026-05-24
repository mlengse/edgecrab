//! Default checkpoint exclude patterns (mirrors hermes-agent `DEFAULT_EXCLUDES`).

/// Built-in deny list written to `store/info/exclude` on first init.
pub const DEFAULT_EXCLUDES: &[&str] = &[
    // Dependency / build output
    "node_modules/",
    "dist/",
    "build/",
    "target/",
    "out/",
    ".next/",
    ".nuxt/",
    // Caches
    "__pycache__/",
    "*.pyc",
    "*.pyo",
    ".cache/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
    "coverage/",
    ".coverage",
    // Virtualenvs
    ".venv/",
    "venv/",
    "env/",
    // VCS
    ".git/",
    ".hg/",
    ".svn/",
    // Worktrees
    ".worktrees/",
    // Native / compiled binaries
    "*.so",
    "*.dylib",
    "*.dll",
    "*.o",
    "*.a",
    "*.jar",
    "*.class",
    "*.exe",
    "*.obj",
    // Media / large binaries
    "*.mp4",
    "*.mov",
    "*.mkv",
    "*.webm",
    "*.zip",
    "*.tar",
    "*.tar.gz",
    "*.tgz",
    "*.7z",
    "*.rar",
    "*.iso",
    // Secrets
    ".env",
    ".env.*",
    ".env.local",
    ".env.*.local",
    // OS junk
    ".DS_Store",
    "Thumbs.db",
    // Logs
    "*.log",
    // Lock files (spec)
    "*.lock",
];

/// Return the exclude file body for the shadow store.
pub fn default_exclude_file_content() -> String {
    DEFAULT_EXCLUDES.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_excludes_cover_build_artifacts() {
        let content = default_exclude_file_content();
        for needle in ["node_modules/", "target/", ".git/", ".venv/", "__pycache__/"] {
            assert!(content.contains(needle), "missing {needle}");
        }
    }
}
