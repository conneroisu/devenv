use crate::nix_internal_log::NixInternalLog;

use regex::Regex;
use std::path::PathBuf;

/// A sum-type of filesystem operations that we can extract from the Nix logs.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// Copied a file to the Nix store.
    CopiedSource { source: PathBuf, target: PathBuf },
    /// Evaluated a Nix file.
    EvaluatedFile { source: PathBuf },
    /// Read a file's contents with `builtins.readFile`.
    ReadFile { source: PathBuf },
    /// Used a tracked devenv string path.
    TrackedPath { source: PathBuf },
}

impl Op {
    /// Extract an `Op` from a `NixInternalLog`.
    pub fn from_internal_log(log: &NixInternalLog) -> Option<Self> {
        lazy_static::lazy_static! {
            static ref EVALUATED_FILE: Regex =
               Regex::new("^evaluating file '(?P<source>.*)'$").expect("invalid regex");
            static ref COPIED_SOURCE: Regex =
                Regex::new("^copied source '(?P<source>.*)' -> '(?P<target>.*)'$").expect("invalid regex");
            static ref READ_FILE: Regex =
                Regex::new("^trace: devenv readFile: '(?P<source>.*)'$").expect("invalid regex");
            static ref TRACKED_PATH: Regex =
                Regex::new("^trace: devenv path: '(?P<source>.*)'$").expect("invalid regex");
        }

        match log {
            NixInternalLog::Msg { msg, .. } => {
                if let Some(matches) = COPIED_SOURCE.captures(msg) {
                    let source = PathBuf::from(&matches["source"]);
                    let target = PathBuf::from(&matches["target"]);
                    Some(Op::CopiedSource { source, target })
                } else if let Some(matches) = EVALUATED_FILE.captures(msg) {
                    let mut source = PathBuf::from(&matches["source"]);
                    // If the evaluated file is a directory, we assume that the file is `default.nix`.
                    if source.is_dir() {
                        source.push("default.nix");
                    }
                    Some(Op::EvaluatedFile { source })
                } else if let Some(matches) = READ_FILE.captures(msg) {
                    let source = PathBuf::from(&matches["source"]);
                    Some(Op::ReadFile { source })
                } else if let Some(matches) = TRACKED_PATH.captures(msg) {
                    let source = PathBuf::from(&matches["source"]);
                    Some(Op::TrackedPath { source })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_log(msg: &str) -> NixInternalLog {
        NixInternalLog::Msg {
            msg: msg.to_string(),
            raw_msg: None,
            level: 1,
        }
    }

    #[test]
    fn test_copied_source() {
        let log = create_log("copied source '/path/to/source' -> '/path/to/target'");
        let op = Op::from_internal_log(&log);
        assert_eq!(
            op,
            Some(Op::CopiedSource {
                source: PathBuf::from("/path/to/source"),
                target: PathBuf::from("/path/to/target"),
            })
        );
    }

    #[test]
    fn test_evaluated_file() {
        let log = create_log("evaluating file '/path/to/file'");
        let op = Op::from_internal_log(&log);
        assert_eq!(
            op,
            Some(Op::EvaluatedFile {
                source: PathBuf::from("/path/to/file"),
            })
        );
    }

    #[test]
    fn test_read_file() {
        let log = create_log("trace: devenv readFile: '/path/to/file'");
        let op = Op::from_internal_log(&log);
        assert_eq!(
            op,
            Some(Op::ReadFile {
                source: PathBuf::from("/path/to/file"),
            })
        );
    }

    #[test]
    fn test_tracked_path() {
        let log = create_log("trace: devenv path: '/path/to/file'");
        let op = Op::from_internal_log(&log);
        assert_eq!(
            op,
            Some(Op::TrackedPath {
                source: PathBuf::from("/path/to/file"),
            })
        );
    }

    #[test]
    fn test_unmatched_log() {
        let log = create_log("some unrelated message");
        let op = Op::from_internal_log(&log);
        assert_eq!(op, None);
    }

    #[test]
    fn test_non_msg_log() {
        let log = NixInternalLog::Stop { id: 1 };
        let op = Op::from_internal_log(&log);
        assert_eq!(op, None);
    }
}