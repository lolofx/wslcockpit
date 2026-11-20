//! Parseurs purs de la sortie de `wsl.exe`, testables sans WSL.

pub mod decode;
pub mod errors;
pub mod parse_list;
pub mod parse_version;

pub use decode::decode_output;
pub use errors::{classify_failure, WslError};
pub use parse_list::{parse_list, DistroEntry, DistroState};
pub use parse_version::parse_wsl_version;
