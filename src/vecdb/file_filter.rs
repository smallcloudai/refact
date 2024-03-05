use std::fs;
use std::io::Read;
use std::path::PathBuf;
use tracing::info;

const LARGE_FILE_SIZE_THRESHOLD: u64 = 10_000_000;
// 10 MB
const SMALL_FILE_SIZE_THRESHOLD: u64 = 10;  // 10 Bytes

const SOURCE_FILE_EXTENSIONS: &[&str] = &[
    "c", "cpp", "cc", "h", "hpp", "cs", "java", "py", "rb", "go", "rs", "swift",
    "php", "js", "jsx", "ts", "tsx", "lua", "pl", "r", "sh", "bat", "cmd", "ps1",
    "m", "kt", "kts", "groovy", "dart", "fs", "fsx", "fsi", "html", "htm", "css",
    "scss", "sass", "less", "json", "xml", "yml", "yaml", "md", "sql", "db", "sqlite",
    "mdf", "cfg", "conf", "ini", "toml", "dockerfile", "ipynb", "rmd", "swift", "java",
    "xml", "kt", "xaml", "unity", "gd", "uproject", "uasset", "asm", "s", "tex",
    "makefile", "mk", "cmake", "gradle",
];

pub fn is_valid_file(path: &PathBuf) -> bool {
    // Check if the path points to a file
    if !path.is_file() {
        info!("path is not a file: {}", path.display());
        return false;
    }

    // Check if the file is in a hidden directory
    if path.ancestors().any(|ancestor| {
        ancestor.file_name()
            .map(|name| name.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
    }) {
        info!("path is in a hidden directory, skipping it: {}", path.display());
        return false;
    }

    // Check if the file is a source file
    if let Some(extension) = path.extension() {
        if !SOURCE_FILE_EXTENSIONS.contains(&extension.to_str().unwrap_or_default()) {
            info!("path has an unsupported extension: {}", path.display());
            return false;
        }
    } else {
        // No extension, not a source file
        info!("path has no extension, skipping it: {}", path.display());
        return false;
    }

    // Check file size
    if let Ok(metadata) = fs::metadata(path) {
        let file_size = metadata.len();
        if file_size < SMALL_FILE_SIZE_THRESHOLD {
            info!("file is too small, skipping: {}", path.display());
            return false;
        }
        if file_size > LARGE_FILE_SIZE_THRESHOLD {
            info!("file is too large, skipping: {}", path.display());
            return false;
        }
    } else {
        // Unable to access file metadata
        info!("unable to access file metadata: {}", path.display());
        return false;
    }

    // Check for read permissions
    if fs::read(&path).is_err() {
        info!("no read permissions on file: {}", path.display());
        return false;
    }

    // Check if the file is not UTF-8
    let mut file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            info!("unable to open file: {}", path.display());
            return false;
        }
    };
    let mut buffer = Vec::new();
    if file.read_to_end(&mut buffer).is_err() {
        info!("unable to read file: {}", path.display());
        return false;
    }
    if String::from_utf8(buffer).is_err() {
        info!("file is not valid utf8: {}", path.display());
        return false;
    }

    true
}
