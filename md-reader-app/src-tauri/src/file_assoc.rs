use std::ffi::OsString;
use std::path::PathBuf;

use tauri::Url;

use crate::file_reader::is_markdown_path;

/// Pick the Markdown file from the process arguments. Used on Windows/Linux
/// (and when the binary is launched with a path from a terminal).
pub fn path_from_args<I>(args: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = OsString>,
{
    args.into_iter()
        .skip(1)
        .map(PathBuf::from)
        .find(|path| is_markdown_path(path))
}

/// Pick the first Markdown file from a macOS "open with" event.
pub fn path_from_urls(urls: &[Url]) -> Option<PathBuf> {
    urls.iter().find_map(|url| {
        if url.scheme() != "file" {
            return None;
        }
        url.to_file_path()
            .ok()
            .filter(|path| is_markdown_path(path))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_markdown_argument() {
        let args = vec![
            OsString::from("/Applications/Markdown Reader.app/Contents/MacOS/app"),
            OsString::from("--some-flag"),
            OsString::from("/tmp/notes.md"),
        ];
        assert_eq!(
            path_from_args(args).unwrap(),
            PathBuf::from("/tmp/notes.md")
        );
    }

    #[test]
    fn ignores_arguments_that_are_not_markdown() {
        let args = vec![
            OsString::from("app"),
            OsString::from("/tmp/notes.txt"),
            OsString::from("-psn_0_12345"),
        ];
        assert!(path_from_args(args).is_none());
    }
}
