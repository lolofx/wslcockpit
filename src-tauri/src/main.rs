#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod settings;
mod tray;

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{Manager, State};
use wsl_core::{
    apply_values, cpu_percent, is_newer, list_distros, now_timestamp, open_terminal, parse_df,
    parse_lxss, parse_vm_sample, powershell, read_lxss, read_values, read_wslconf, run_checked,
    shutdown_all, start_distro, start_once, stop_distro, vhdx_path, write_with_backup,
    write_wslconf, wslconf_needs_start, DistroEntry, DistroState, WslError, WslExe, DISTRO_FIELDS,
    GLOBAL_FIELDS, VM_SCRIPT,
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
async fn set_default_distro(name: String) -> Result<(), String> {
    wsl_core::set_default(&WslExe, &name).map_err(msg)
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
fn policies() -> settings::Aliases {
    settings::load_file("policies.json")
}

#[tauri::command]
fn set_policy(name: String, policy: String) -> Result<settings::Aliases, String> {
    if !settings::valid_policy(&policy) {
        return Err("Politique inconnue.".into());
    }
    let mut p = settings::load_file("policies.json");
    if policy == "manual" {
        p.remove(&name);
    } else {
        p.insert(name, policy);
    }
    settings::save_file("policies.json", &p)?;
    Ok(p)
}

/// À l'ouverture : démarre les distros arrêtées dont la politique l'exige.
#[tauri::command]
async fn apply_policies() -> Result<(), String> {
    let p = settings::load_file("policies.json");
    let list = list_distros(&WslExe).map_err(msg)?;
    for d in list.iter().filter(|d| d.state != DistroState::Running) {
        match p.get(&d.name).map(String::as_str) {
            Some("keepalive") => start_distro(&d.name).map_err(msg)?,
            Some("start") => start_once(&d.name).map_err(msg)?,
            _ => {}
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct Monitor {
    ram_bytes: u64,
    cpu_percent: Option<f64>,
    /// nom -> (utilisé, total) en octets, distros démarrées seulement
    disks: HashMap<String, (u64, u64)>,
}

#[derive(Default)]
struct CpuState(Mutex<Option<(Instant, f64)>>);

/// Chiffres de la VM. Renvoie `None` si la fenêtre n'est pas visible (RM08) ou si WSL est arrêté.
#[tauri::command]
async fn monitor(
    app: tauri::AppHandle,
    cpu: State<'_, CpuState>,
) -> Result<Option<Monitor>, String> {
    let visible = app
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
        .unwrap_or(false);
    if !visible {
        return Ok(None);
    }
    let list = list_distros(&WslExe).map_err(msg)?;
    let running: Vec<_> = list
        .iter()
        .filter(|d| d.state == DistroState::Running)
        .collect();
    if running.is_empty() {
        *cpu.0.lock().unwrap() = None;
        return Ok(None);
    }
    let Some(sample) = parse_vm_sample(&powershell(VM_SCRIPT)) else {
        return Ok(None);
    };
    let now = Instant::now();
    let percent = {
        let mut prev = cpu.0.lock().unwrap();
        let p =
            prev.and_then(|(t, c)| cpu_percent(c, &sample, now.duration_since(t).as_secs_f64()));
        *prev = Some((now, sample.cpu_secs));
        p
    };
    let mut disks = HashMap::new();
    for d in running {
        if let Ok(out) = run_checked(&WslExe, &["-d", &d.name, "--exec", "df", "-k", "/"]) {
            if let Some(v) = parse_df(&out) {
                disks.insert(d.name.clone(), v);
            }
        }
    }
    Ok(Some(Monitor {
        ram_bytes: sample.ram_bytes,
        cpu_percent: percent,
        disks,
    }))
}

#[derive(Serialize)]
struct Update {
    version: String,
    url: String,
}

/// Dernière release GitHub si elle est plus récente ; `None` sans bruit si hors ligne ou aucune release.
#[tauri::command]
async fn check_update() -> Option<Update> {
    let out = powershell(
        "try { $r = Invoke-RestMethod -UseBasicParsing -TimeoutSec 5 \
         -Uri 'https://api.github.com/repos/lolofx/wslcockpit/releases/latest' \
         -Headers @{'User-Agent'='WSLCockpit'}; \
         \"$($r.tag_name) $($r.html_url)\" } catch {}",
    );
    let mut it = out.split_whitespace();
    let (tag, url) = (it.next()?, it.next()?);
    (url.starts_with("https://github.com/lolofx/wslcockpit/")
        && is_newer(tag, env!("CARGO_PKG_VERSION")))
    .then(|| Update {
        version: tag.to_string(),
        url: url.to_string(),
    })
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/lolofx/wslcockpit/") {
        return Err("Lien refusé.".into());
    }
    std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &url])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
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

fn wslconfig_path() -> Result<std::path::PathBuf, String> {
    let home = std::env::var_os("USERPROFILE").ok_or("Dossier utilisateur introuvable.")?;
    Ok(std::path::PathBuf::from(home).join(".wslconfig"))
}

/// Contenu de `%USERPROFILE%\.wslconfig` (vide s'il n'existe pas).
#[tauri::command]
fn read_wslconfig() -> Result<String, String> {
    match std::fs::read_to_string(wslconfig_path()?) {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("Lecture de .wslconfig impossible : {e}")),
    }
}

/// Écrit `.wslconfig` après sauvegarde horodatée ; renvoie le chemin de la sauvegarde.
#[tauri::command]
fn write_wslconfig(text: String) -> Result<Option<String>, String> {
    write_with_backup(&wslconfig_path()?, &text, &now_timestamp())
        .map(|b| b.map(|p| p.display().to_string()))
        .map_err(|e| format!("Écriture de .wslconfig impossible : {e}"))
}

/// Refuse de démarrer la distro sans accord : erreur `NEEDS_START`.
fn guard_start(distro: &str, allow_start: bool) -> Result<(), String> {
    if allow_start {
        return Ok(());
    }
    let list = list_distros(&WslExe).map_err(msg)?;
    if wslconf_needs_start(&list, distro) {
        Err("NEEDS_START".into())
    } else {
        Ok(())
    }
}

#[tauri::command]
async fn read_distro_conf(distro: String, allow_start: bool) -> Result<String, String> {
    guard_start(&distro, allow_start)?;
    read_wslconf(&WslExe, &distro).map_err(msg)
}

#[tauri::command]
async fn write_distro_conf(distro: String, text: String, allow_start: bool) -> Result<(), String> {
    guard_start(&distro, allow_start)?;
    write_wslconf(&distro, &text, &now_timestamp()).map_err(msg)
}

#[derive(Serialize)]
struct ConfigView {
    values: Vec<(String, String)>,
}

/// Valeurs du `.wslconfig` (distro = None) ou du `wsl.conf` d'une distro.
#[tauri::command]
async fn config_load(distro: Option<String>, allow_start: bool) -> Result<ConfigView, String> {
    let (text, fields) = match &distro {
        None => (read_wslconfig()?, &GLOBAL_FIELDS[..]),
        Some(d) => {
            guard_start(d, allow_start)?;
            (read_wslconf(&WslExe, d).map_err(msg)?, &DISTRO_FIELDS[..])
        }
    };
    Ok(ConfigView {
        values: read_values(&text, fields),
    })
}

/// Valide puis enregistre. Erreur de validation : JSON `[[id, message], ...]` préfixé `INVALID:`.
#[tauri::command]
async fn config_save(
    distro: Option<String>,
    values: Vec<(String, String)>,
    allow_start: bool,
) -> Result<(), String> {
    let invalid = |e: Vec<(String, String)>| {
        format!("INVALID:{}", serde_json::to_string(&e).unwrap_or_default())
    };
    match distro {
        None => {
            let new = apply_values(&read_wslconfig()?, &GLOBAL_FIELDS, &values).map_err(invalid)?;
            write_wslconfig(new).map(|_| ())
        }
        Some(d) => {
            guard_start(&d, allow_start)?;
            let old = read_wslconf(&WslExe, &d).map_err(msg)?;
            let new = apply_values(&old, &DISTRO_FIELDS, &values).map_err(invalid)?;
            write_wslconf(&d, &new, &now_timestamp()).map_err(msg)
        }
    }
}

fn main() {
    tauri::Builder::default()
        .manage(CpuState::default())
        .setup(|app| tray::setup(app).map_err(Into::into))
        .invoke_handler(tauri::generate_handler![
            distros,
            start,
            stop,
            set_default_distro,
            restart,
            terminal,
            shutdown,
            aliases,
            set_alias,
            autostart_get,
            autostart_set,
            policies,
            set_policy,
            apply_policies,
            monitor,
            check_update,
            open_url,
            read_wslconfig,
            write_wslconfig,
            read_distro_conf,
            write_distro_conf,
            config_load,
            config_save
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de WSLCockpit");
}
