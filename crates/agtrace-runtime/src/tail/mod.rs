//! Offset-based incremental tailing (design §4.1): a `FileCursor` per agent file
//! that feeds only newly appended complete lines to the provider's `LogDecoder`.
//!
//! Skeleton only — implemented by the tailer PR.
