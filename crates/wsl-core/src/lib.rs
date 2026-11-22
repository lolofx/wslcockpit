//! Parseurs purs de la sortie de `wsl.exe`, testables sans WSL.

pub mod decode;
pub mod errors;
pub mod exec;
pub mod monitor;
pub mod parse_list;
pub mod parse_lxss;
pub mod parse_version;
pub mod version_check;

pub use decode::decode_output;
pub use errors::{classify_failure, WslError};
pub use exec::{
    list_distros, open_terminal, powershell, read_lxss, run_checked, shutdown_all, start_distro,
    start_once, stop_distro, RawOutput, WslExe, WslRunner,
};
pub use monitor::{cpu_percent, parse_df, parse_vm_sample, VmSample, VM_SCRIPT};
pub use parse_list::{parse_list, DistroEntry, DistroState};
pub use parse_lxss::{parse_lxss, vhdx_path, LxssEntry};
pub use parse_version::parse_wsl_version;
pub use version_check::{is_newer, parse_semver};
