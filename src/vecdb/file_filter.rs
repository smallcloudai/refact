use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const LARGE_FILE_SIZE_THRESHOLD: u64 = 10_000_000; // 10 MB
const SMALL_FILE_SIZE_THRESHOLD: u64 = 5;          // 5 Bytes

const SOURCE_FILE_EXTENSIONS: &[&str] = &[
    "c", "cpp", "cc", "h", "hpp", "cs", "java", "py", "rb", "go", "rs", "swift",
    "php", "js", "jsx", "ts", "tsx", "lua", "pl", "r", "sh", "bat", "cmd", "ps1",
    "m", "kt", "kts", "groovy", "dart", "fs", "fsx", "fsi", "html", "htm", "css",
    "scss", "sass", "less", "json", "xml", "yml", "yaml", "md", "sql", "db", "sqlite",
    "mdf", "cfg", "conf", "ini", "toml", "dockerfile", "ipynb", "rmd", "swift", "java",
    "xml", "kt", "xaml", "unity", "gd", "uproject", "uasset", "asm", "s", "tex",
    "makefile", "mk", "cmake", "gradle",
];

pub fn is_valid_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !path.is_file() {
        return Err("Path is not a file".into());
    }

    if path.ancestors().any(|ancestor| {
        ancestor.file_name()
            .map(|name| name.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
    }) {
        return Err("Parent dir stars with a dot".into());
    }

    if let Some(extension) = path.extension() {
        if !SOURCE_FILE_EXTENSIONS.contains(&extension.to_str().unwrap_or_default()) {
            return Err(format!("Unsupported file extension {:?}", extension).into());
        }
    } else {
        return Err("File has no extension".into());
    }

    if let Ok(metadata) = fs::metadata(path) {
        let file_size = metadata.len();
        if file_size < SMALL_FILE_SIZE_THRESHOLD {
            return Err("File size is too small".into());
        }
        if file_size > LARGE_FILE_SIZE_THRESHOLD {
            return Err("File size is too large".into());
        }
        let permissions = metadata.permissions();
        if permissions.mode() & 0o400 == 0 {
            return Err("File has no read permissions".into());
        }
    } else {
        return Err("Unable to access file metadata".into());
    }
    Ok(())
}

