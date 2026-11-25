//! Parseurs purs de la sortie de `wsl.exe`, testables sans WSL.

pub mod config_edit;
pub mod config_io;
pub mod config_validate;
pub mod decode;
pub mod errors;
pub mod exec;
pub mod git;
pub mod ini;
pub mod monitor;
pub mod online;
pub mod ops;
pub mod parse_list;
pub mod parse_lxss;
pub mod parse_version;
pub mod version_check;

pub use config_edit::{apply_values, read_values, DISTRO_FIELDS, GLOBAL_FIELDS};
pub use config_io::{
    backup_name, now_timestamp, read_wslconf, timestamp, write_with_backup, write_wslconf,
    wslconf_needs_start,
};
pub use config_validate::{validate, Field};
pub use decode::decode_output;
pub use errors::{classify_failure, WslError};
pub use exec::{
    list_distros, open_terminal, powershell, read_lxss, run_checked, set_default, shutdown_all,
    start_distro, start_once, stop_distro, unregister_distro, RawOutput, WslExe, WslRunner,
};
pub use git::{
    git_command, parse_root, parse_status, safe_rel, windows_path, GitFile, GitStatus, Root,
    STATUS_ARGS,
};
pub use ini::Ini;
pub use monitor::{cpu_percent, parse_df, parse_vm_sample, VmSample, VM_SCRIPT};
pub use online::{custom_install_supported, install_args, parse_online, OnlineDistro};
pub use ops::{
    check_new_name, check_tar_target, check_target_dir, convert_args, export_args, import_args,
    run_cancellable, sparse_args, sparse_supported,
};
pub use parse_list::{parse_list, DistroEntry, DistroState};
pub use parse_lxss::{parse_lxss, vhdx_path, LxssEntry};
pub use parse_version::parse_wsl_version;
pub use version_check::{is_newer, parse_semver};
