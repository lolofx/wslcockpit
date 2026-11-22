#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;

use serde::Serialize;
use wsl_core::{
    list_distros, open_terminal, shutdown_all, start_distro, stop_distro, DistroEntry, DistroState,
    WslError, WslExe,
};

#[derive(Serialize)]
struct DistroDto {
    name: String,
    state: String,
    version: u8,
    is_default: bool,
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
    list_distros(&WslExe)
        .map(|v| v.into_iter().map(DistroDto::from).collect())
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
async fn shutdown() -> Result<(), String> {
    shutdown_all(&WslExe).map_err(msg)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            distros, start, stop, restart, terminal, shutdown, aliases, set_alias
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de WSLCockpit");
}
