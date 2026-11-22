#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod settings;
mod tray;

use serde::Serialize;
use wsl_core::{
    list_distros, open_terminal, parse_lxss, read_lxss, shutdown_all, start_distro, stop_distro,
    vhdx_path, DistroEntry, DistroState, WslError, WslExe,
};

#[derive(Serialize)]
struct DistroDto {
    name: String,
    state: String,
    version: u8,
    is_default: bool,
    vhdx_bytes: Option<u64>,
}

impl From<DistroEntry> for DistroDto {
    fn from(d: DistroEntry) -> Self {
        let state = match d.state {
            DistroState::Running => "running".to_string(),
            DistroState::Stopped => "stopped".to_string(),
            DistroState::Installing => "installing".to_string(),
            DistroState::Other(raw) => raw,
        };
        Self {
            name: d.name,
            state,
            version: d.version,
            is_default: d.is_default,
            vhdx_bytes: None,
        }
    }
}

fn msg(e: WslError) -> String {
    match e {
        WslError::NotInstalled => "WSL_ABSENT".into(),
        WslError::DistroNotFound => "Cette distribution n'existe plus.".into(),
        WslError::Other { code, message } => format!("wsl.exe a échoué (code {code}) : {message}"),
    }
}

#[tauri::command]
async fn distros() -> Result<Vec<DistroDto>, String> {
    let sizes: std::collections::HashMap<String, u64> = parse_lxss(&read_lxss())
        .into_iter()
        .filter_map(|e| {
            let size = std::fs::metadata(vhdx_path(&e.base_path?)).ok()?.len();
            Some((e.name?, size))
        })
        .collect();
    list_distros(&WslExe)
        .map(|v| {
            v.into_iter()
                .map(|d| {
                    let mut dto = DistroDto::from(d);
                    dto.vhdx_bytes = sizes.get(&dto.name).copied();
                    dto
                })
                .collect()
        })
        .map_err(msg)
}

#[tauri::command]
async fn start(name: String) -> Result<(), String> {
    start_distro(&name).map_err(msg)
}

#[tauri::command]
async fn stop(name: String) -> Result<(), String> {
    stop_distro(&WslExe, &name).map_err(msg)
}

#[tauri::command]
async fn restart(name: String) -> Result<(), String> {
    stop_distro(&WslExe, &name).map_err(msg)?;
    start_distro(&name).map_err(msg)
}

#[tauri::command]
async fn terminal(name: String) -> Result<(), String> {
    open_terminal(&name).map_err(msg)
}

#[tauri::command]
fn aliases() -> settings::Aliases {
    settings::load()
}

#[tauri::command]
fn set_alias(name: String, alias: String) -> Result<settings::Aliases, String> {
    let a = settings::apply(settings::load(), &name, &alias);
    settings::save(&a)?;
    Ok(a)
}

#[tauri::command]
fn autostart_get() -> bool {
    autostart::is_enabled()
}

#[tauri::command]
fn autostart_set(on: bool) -> Result<bool, String> {
    autostart::set_enabled(on)?;
    Ok(autostart::is_enabled())
}

#[tauri::command]
async fn shutdown() -> Result<(), String> {
    shutdown_all(&WslExe).map_err(msg)
}

fn main() {
    tauri::Builder::default()
        .setup(|app| tray::setup(app).map_err(Into::into))
        .invoke_handler(tauri::generate_handler![
            distros,
            start,
            stop,
            restart,
            terminal,
            shutdown,
            aliases,
            set_alias,
            autostart_get,
            autostart_set
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de WSLCockpit");
}
