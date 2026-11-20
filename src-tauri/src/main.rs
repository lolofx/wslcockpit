#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use wsl_core::{list_distros, DistroEntry, DistroState, WslExe};

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

#[tauri::command]
fn distros() -> Result<Vec<DistroDto>, String> {
    list_distros(&WslExe)
        .map(|v| v.into_iter().map(DistroDto::from).collect())
        .map_err(|e| format!("{e:?}"))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![distros])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de WSLCockpit");
}
