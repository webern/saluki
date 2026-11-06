//! Line-level scan of crate files for terms.
//!
//! Unlike [`crate::symbols`], this scan reads text, so comments, doc text, string literals, fixtures, manifests, and
//! embedded assets in any file type count. Use it for boundaries that must hold in everything a crate ships, such as
//! keeping product knowledge out of a generic crate.

use std::{fs, io, path::Path};

use snafu::{ResultExt as _, Snafu};

/// An error reading a crate's files.
#[derive(Debug, Snafu)]
#[snafu(context(suffix(Snafu)))]
pub enum Error {
    /// The crate has no regular `src/lib.rs`.
    #[snafu(display("{path}: required crate source file is missing or not a regular file"))]
    MissingLib {
        /// The expected path, relative to the scan root.
        path: String,
    },

    /// A file or directory couldn't be inspected or read, including a file that isn't UTF-8.
    #[snafu(display("{path}: {source}"))]
    Read {
        /// The path, relative to the scan root.
        path: String,
        /// The underlying error.
        source: io::Error,
    },

    /// A directory entry's name isn't UTF-8.
    #[snafu(display("{path}: non-UTF-8 file name"))]
    NonUtf8Name {
        /// The directory holding the entry, relative to the scan root.
        path: String,
    },

    /// An entry is neither a regular file nor a directory, such as a symbolic link or socket.
    #[snafu(display("{path}: expected a regular file or directory"))]
    UnsupportedEntry {
        /// The path, relative to the scan root.
        path: String,
    },
}

/// A term to find in a line of text.
#[derive(Clone, Copy, Debug)]
pub enum Term {
    /// The text, case-insensitively, anywhere in the line. The text must be lowercase.
    Text(&'static str),

    /// The identifier as a whole token. Identifier boundaries use ASCII letters, digits, and underscores.
    Ident(&'static str),

    /// The identifier as a whole token on a line that also contains `cfg`.
    CfgIdent(&'static str),

    /// The number as a whole token. Number boundaries use digits alone.
    Number(&'static str),
}

impl Term {
    /// Returns the term if `line` contains it.
    pub fn find_in(&self, line: &str) -> Option<&'static str> {
        match self {
            Self::Text(term) if line.to_ascii_lowercase().contains(term) => Some(term),
            Self::Ident(term) if matches_token(line, term, |byte| byte.is_ascii_alphanumeric() || byte == b'_') => {
                Some(term)
            }
            Self::CfgIdent(term) if line.contains("cfg") => Self::Ident(term).find_in(line),
            Self::Number(term) if matches_token(line, term, |byte| byte.is_ascii_digit()) => Some(term),
            _ => None,
        }
    }
}

fn matches_token(line: &str, term: &str, is_token_byte: fn(u8) -> bool) -> bool {
    let bytes = line.as_bytes();
    line.match_indices(term).any(|(start, _)| {
        let end = start + term.len();
        (start == 0 || !is_token_byte(bytes[start - 1])) && (end == bytes.len() || !is_token_byte(bytes[end]))
    })
}

/// A file's scan-root-relative path, with `/` separators, and its text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceText {
    /// The path relative to the scan root.
    pub path: String,

    /// The file's contents.
    pub text: String,
}

impl SourceText {
    /// Creates a file from its path and contents.
    pub fn new(path: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            text: text.into(),
        }
    }
}

/// A set of terms that no line may contain, with the lines exempt from the check.
pub struct TermRule {
    /// The terms no line may contain.
    pub terms: &'static [Term],

    /// Exempt lines as `(path, trimmed line)` pairs. A line is exempt only when both its path and its trimmed
    /// text match exactly.
    pub exact_exemptions: &'static [(&'static str, &'static str)],
}

impl TermRule {
    /// Returns one `path:line: names term, ...` entry for each line that contains a term, ordered by path, then
    /// line.
    pub fn violations(&self, files: &[SourceText]) -> Vec<String> {
        let mut files: Vec<_> = files.iter().collect();
        files.sort_unstable_by_key(|file| &file.path);
        let mut problems = Vec::new();
        for file in files {
            for (index, line) in file.text.lines().enumerate() {
                if self.exact_exemptions.contains(&(file.path.as_str(), line.trim())) {
                    continue;
                }
                let terms: Vec<_> = self.terms.iter().filter_map(|term| term.find_in(line)).collect();
                if !terms.is_empty() {
                    problems.push(format!("{}:{}: names {}", file.path, index + 1, terms.join(", ")));
                }
            }
        }
        problems
    }
}

/// Reads every file under the crate directory `crate_dir`, recursively and in path order, excluding directories
/// named `target`.
///
/// `crate_dir` is relative to `root` and uses `/` separators. Returned paths are relative to `root`.
///
/// # Errors
///
/// Returns an error if the crate lacks `Cargo.toml` or `src/lib.rs`, or if any entry is unreadable, isn't UTF-8,
/// or is neither a regular file nor a directory.
pub fn read_crate(root: &Path, crate_dir: &str) -> Result<Vec<SourceText>, Error> {
    let lib = format!("{crate_dir}/src/lib.rs");
    if !root.join(&lib).is_file() {
        return MissingLibSnafu { path: lib }.fail();
    }

    let manifest = format!("{crate_dir}/Cargo.toml");
    fs::symlink_metadata(root.join(&manifest)).context(ReadSnafu { path: manifest })?;

    let mut pending = vec![crate_dir.to_string()];
    let mut files = Vec::new();
    while let Some(relative) = pending.pop() {
        let path = root.join(&relative);
        let metadata = fs::symlink_metadata(&path).context(ReadSnafu { path: &relative })?;
        if metadata.is_dir() {
            if path.ends_with("target") {
                continue;
            }
            for entry in fs::read_dir(&path).context(ReadSnafu { path: &relative })? {
                let entry = entry.context(ReadSnafu { path: &relative })?;
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    return NonUtf8NameSnafu { path: relative }.fail();
                };
                pending.push(format!("{relative}/{name}"));
            }
        } else if metadata.is_file() {
            let text = fs::read_to_string(&path).context(ReadSnafu { path: &relative })?;
            files.push(SourceText { path: relative, text });
        } else {
            return UnsupportedEntrySnafu { path: relative }.fail();
        }
    }
    files.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

/// Returns `path` relative to `root`, with `/` separators.
///
/// # Panics
///
/// Panics if `path` isn't under `root` or isn't UTF-8.
pub fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("path must be under the root")
        .iter()
        .map(|component| component.to_str().expect("path must be UTF-8"))
        .collect::<Vec<_>>()
        .join("/")
}
