//! Parseurs purs de la sortie de `wsl.exe`, testables sans WSL.

pub mod decode;
pub mod errors;
pub mod exec;
pub mod parse_list;
pub mod parse_lxss;
pub mod parse_version;

pub use decode::decode_output;
pub use errors::{classify_failure, WslError};
pub use exec::{
    list_distros, open_terminal, run_checked, shutdown_all, start_distro, stop_distro, RawOutput,
    WslExe, WslRunner,
};
pub use parse_list::{parse_list, DistroEntry, DistroState};
pub use parse_lxss::{parse_lxss, vhdx_path, LxssEntry};
pub use parse_version::parse_wsl_version;
