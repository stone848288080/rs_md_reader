use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// File extensions the reader accepts.
pub const MARKDOWN_EXTENSIONS: [&str; 2] = ["md", "markdown"];

/// Payload returned to the frontend after a file has been opened.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedFile {
    pub path: String,
    pub name: String,
    pub content: String,
    /// Directory that relative image paths are resolved against.
    pub base_dir: String,
    pub size: u64,
}

/// Internal result: the IPC payload plus the canonical directory that must be
/// whitelisted for the asset protocol.
#[derive(Debug)]
pub struct LoadedFile {
    pub file: OpenedFile,
    pub canonical_base_dir: PathBuf,
}

pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            MARKDOWN_EXTENSIONS
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(ext))
        })
        .unwrap_or(false)
}

pub fn read_markdown(path: &Path) -> Result<LoadedFile, String> {
    if !is_markdown_path(path) {
        return Err("仅支持打开 .md 或 .markdown 文件".to_string());
    }

    let canonical = fs::canonicalize(path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => "文件不存在或已被移动".to_string(),
        std::io::ErrorKind::PermissionDenied => "没有权限读取该文件".to_string(),
        _ => format!("无法打开文件：{err}"),
    })?;

    let metadata = fs::metadata(&canonical).map_err(|err| format!("无法读取文件信息：{err}"))?;
    if !metadata.is_file() {
        return Err("所选路径不是一个文件".to_string());
    }

    let name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "文件名无效".to_string())?
        .to_string();

    let bytes = fs::read(&canonical).map_err(|err| format!("无法读取文件：{err}"))?;
    let mut content =
        String::from_utf8(bytes).map_err(|_| "文件不是有效的 UTF-8 文本".to_string())?;
    if content.starts_with('\u{feff}') {
        content.remove(0);
    }

    let base_dir = canonical
        .parent()
        .ok_or_else(|| "无法确定文件所在目录".to_string())?;

    Ok(LoadedFile {
        file: OpenedFile {
            // On Windows `canonicalize` returns a verbatim path (`\\?\C:\...`).
            // Strip the prefix before handing the path to the WebView.
            path: display_path(&canonical).to_string_lossy().into_owned(),
            name,
            content,
            base_dir: display_path(base_dir).to_string_lossy().into_owned(),
            size: metadata.len(),
        },
        canonical_base_dir: base_dir.to_path_buf(),
    })
}

#[cfg(windows)]
fn display_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(stripped) => PathBuf::from(stripped),
        None => path.to_path_buf(),
    }
}

#[cfg(not(windows))]
fn display_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn accepts_markdown_extensions_case_insensitively() {
        assert!(is_markdown_path(Path::new("README.md")));
        assert!(is_markdown_path(Path::new("notes.MARKDOWN")));
        assert!(!is_markdown_path(Path::new("notes.txt")));
        assert!(!is_markdown_path(Path::new("notes")));
    }

    #[test]
    fn reads_content_metadata_and_base_dir() {
        let dir = std::env::temp_dir().join("md-reader-test-read");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.md");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "# Hello\n\n| a | b |\n| - | - |\n| 1 | 2 |").unwrap();

        let loaded = read_markdown(&path).unwrap();
        assert_eq!(loaded.file.name, "sample.md");
        assert!(loaded.file.content.starts_with("# Hello"));
        assert_eq!(
            loaded.file.base_dir,
            fs::canonicalize(&dir).unwrap().to_string_lossy()
        );
        assert!(loaded.file.size > 0);

        fs::remove_file(&path).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn rejects_non_markdown_files() {
        let path = std::env::temp_dir().join("md-reader-test.txt");
        fs::write(&path, "not markdown").unwrap();
        let error = read_markdown(&path).unwrap_err();
        assert!(error.contains(".md"));
        fs::remove_file(&path).unwrap();
    }
}
