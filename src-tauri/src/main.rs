#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod ops;
mod project;
mod pty;
mod settings;
mod tray;

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{Emitter, Manager, State};
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
async fn delete_distro(name: String, typed: String) -> Result<(), String> {
    wsl_core::unregister_distro(&WslExe, &name, &typed).map_err(msg)?;
    // Nettoie les réglages locaux de cette distro.
    for file in ["aliases.json", "policies.json"] {
        let a = settings::apply(settings::load_file(file), &name, "");
        let _ = settings::save_file(file, &a);
    }
    Ok(())
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

#[derive(Serialize)]
struct PtyOpened {
    replay: String,
    end: u64,
    alive: bool,
}

#[derive(Serialize, Clone)]
struct PtyData {
    id: String,
    data: String,
    end: u64,
}

/// Seuls ces programmes peuvent être lancés dans un terminal intégré (jamais un chemin arbitraire).
fn allowed_program(p: &str) -> bool {
    matches!(
        p.to_ascii_lowercase().as_str(),
        "wsl.exe" | "powershell.exe"
    )
}

/// Variables que le front peut injecter : le suivi du dossier courant (OSC 7) et la commande d'un lanceur.
fn allowed_env(name: &str) -> bool {
    matches!(name, "PROMPT_COMMAND" | "WSLC_CMD")
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn pty_open(
    app: tauri::AppHandle,
    st: State<pty::PtyState>,
    id: String,
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
    env: Option<Vec<(String, String)>>,
) -> Result<PtyOpened, String> {
    if !allowed_program(&program) {
        return Err(format!("Programme non autorisé : {program}"));
    }
    let env = env.unwrap_or_default();
    if let Some((k, _)) = env.iter().find(|(k, _)| !allowed_env(k)) {
        return Err(format!("Variable non autorisée : {k}"));
    }
    let sink: pty::Sink = std::sync::Arc::new(move |ev| match ev {
        pty::Event::Data { id, bytes, end } => {
            use base64::Engine;
            let data = base64::engine::general_purpose::STANDARD.encode(bytes);
            let _ = app.emit("pty-data", PtyData { id, data, end });
        }
        pty::Event::Exit { id } => {
            let _ = app.emit("pty-exit", id);
        }
    });
    let o = st.open(
        &id,
        &pty::Spec {
            program,
            args,
            cwd,
            cols,
            rows,
            env,
        },
        sink,
    )?;
    use base64::Engine;
    Ok(PtyOpened {
        replay: base64::engine::general_purpose::STANDARD.encode(o.replay),
        end: o.end,
        alive: o.alive,
    })
}

#[tauri::command]
fn pty_write(st: State<pty::PtyState>, id: String, data: String) -> Result<(), String> {
    st.write(&id, data.as_bytes())
}

#[tauri::command]
fn pty_resize(st: State<pty::PtyState>, id: String, cols: u16, rows: u16) -> Result<(), String> {
    st.resize(&id, cols, rows)
}

#[tauri::command]
fn pty_close(st: State<pty::PtyState>, id: String) {
    st.close(&id);
}

/// Dispositions des espaces de travail (layouts.json) : document JSON opaque pour le Rust,
/// validé et réparé côté front (parseLayout). Écriture atomique.
#[tauri::command]
fn layouts_load() -> serde_json::Value {
    settings::settings_path("layouts.json")
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
fn layouts_save(doc: serde_json::Value) -> Result<(), String> {
    let p = settings::settings_path("layouts.json")?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json)
        .map_err(|e| format!("Écriture des dispositions impossible : {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("Écriture des dispositions impossible : {e}"))
}

/// Réglages JSON structurés (icônes, lanceurs...) : liste blanche de fichiers, écriture atomique.
const STORES: [&str; 2] = ["icons.json", "launchers.json"];

fn store_name(file: &str) -> Result<&str, String> {
    STORES
        .iter()
        .copied()
        .find(|f| *f == file)
        .ok_or_else(|| format!("Fichier de réglages inconnu : {file}"))
}

#[tauri::command]
fn store_load(file: String) -> Result<serde_json::Value, String> {
    let p = settings::settings_path(store_name(&file)?)?;
    Ok(std::fs::read_to_string(p)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
fn store_save(file: String, doc: serde_json::Value) -> Result<(), String> {
    let p = settings::settings_path(store_name(&file)?)?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| format!("Écriture des réglages impossible : {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("Écriture des réglages impossible : {e}"))
}

/// Outils présents dans la distro (shell de connexion interactif : PATH de nvm, ~/.local/bin...).
/// Les noms sont validés (lettres, chiffres, - et _) avant d'être substitués dans `command -v`.
#[tauri::command]
fn detect_tools(distro: String, names: Vec<String>) -> Vec<String> {
    names
        .into_iter()
        .filter(|n| {
            !n.is_empty()
                && n.len() < 40
                && n.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        .filter(|n| {
            let mut cmd = std::process::Command::new("wsl.exe");
            cmd.args([
                "-d",
                &distro,
                "-e",
                "sh",
                "-c",
                "exec $SHELL -lic \"command -v $0\" >/dev/null 2>&1",
                n,
            ]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000);
            }
            cmd.stdin(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
        .collect()
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

#[derive(Serialize)]
struct WslUpdate {
    current: Option<String>,
    /// `None` : hors ligne ou GitHub injoignable.
    latest: Option<String>,
    newer: bool,
}

fn dotted(v: &[u32]) -> String {
    let n = v.len().min(3);
    v[..n]
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

/// Version de WSL installée et dernière release publique de microsoft/WSL.
#[tauri::command]
async fn wsl_update_check() -> WslUpdate {
    let current = ops::wsl_version().map(|v| dotted(&v));
    let out = powershell(
        "try { (Invoke-RestMethod -UseBasicParsing -TimeoutSec 5 \
         -Uri 'https://api.github.com/repos/microsoft/WSL/releases/latest' \
         -Headers @{'User-Agent'='WSLCockpit'}).tag_name } catch {}",
    );
    let latest = out
        .split_whitespace()
        .next()
        .filter(|t| wsl_core::parse_semver(t).is_some())
        .map(|t| t.trim_start_matches('v').to_string());
    let newer = match (&latest, &current) {
        (Some(l), Some(c)) => is_newer(l, c),
        _ => false,
    };
    WslUpdate {
        current,
        latest,
        newer,
    }
}

#[tauri::command]
async fn op_wsl_update(st: State<'_, ops::OpState>) -> Result<(), String> {
    ops::wsl_update(&st)
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

/// Ouvre une adresse web dans le navigateur du système (http ou https seulement, sans espace ni caractère de contrôle).
fn is_web_url(u: &str) -> bool {
    (u.starts_with("https://") || u.starts_with("http://"))
        && u.len() < 2048
        && !u
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '"')
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    if !is_web_url(&url) {
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

#[tauri::command]
fn op_default_base() -> String {
    if let Some(b) = settings::load_file("app.json").get("default_base") {
        if !b.trim().is_empty() {
            return b.clone();
        }
    }
    std::env::var("USERPROFILE")
        .map(|h| format!("{h}\\WSL"))
        .unwrap_or_default()
}

/// Dossier par défaut des nouvelles distributions (vide = valeur d'origine). Enregistré dans app.json.
#[tauri::command]
fn op_set_default_base(path: String) -> Result<String, String> {
    let mut a = settings::load_file("app.json");
    let p = path.trim().trim_end_matches('\\').to_string();
    if p.is_empty() {
        a.remove("default_base");
    } else {
        a.insert("default_base".into(), p);
    }
    settings::save_file("app.json", &a)?;
    Ok(op_default_base())
}

#[tauri::command]
fn op_status(st: State<ops::OpState>) -> ops::Status {
    st.status()
}

#[tauri::command]
fn op_cancel(st: State<ops::OpState>) {
    st.cancel();
}

#[tauri::command]
async fn op_export(st: State<'_, ops::OpState>, name: String, tar: String) -> Result<(), String> {
    ops::export(&st, name, tar)
}

#[tauri::command]
async fn op_import(
    st: State<'_, ops::OpState>,
    name: String,
    dir: String,
    tar: String,
) -> Result<(), String> {
    ops::import(&st, name, dir, tar)
}

#[tauri::command]
async fn op_clone(
    st: State<'_, ops::OpState>,
    source: String,
    new_name: String,
    dir: String,
) -> Result<(), String> {
    ops::clone(&st, source, new_name, dir)
}

#[derive(Serialize)]
struct Online {
    name: String,
    friendly: String,
}

/// Catalogue des distributions installables (réseau requis).
#[tauri::command]
async fn list_online() -> Result<Vec<Online>, String> {
    let text = wsl_core::run_checked(&WslExe, &["--list", "--online"]).map_err(msg)?;
    Ok(wsl_core::parse_online(&text)
        .into_iter()
        .map(|d| Online {
            name: d.name,
            friendly: d.friendly,
        })
        .collect())
}

#[tauri::command]
fn custom_install_available() -> Result<(), String> {
    wsl_core::custom_install_supported(ops::wsl_version().as_deref())
}

#[tauri::command]
async fn op_create(
    st: State<'_, ops::OpState>,
    distro: String,
    name: Option<String>,
    dir: Option<String>,
) -> Result<(), String> {
    ops::create(&st, distro, name, dir)
}

#[tauri::command]
fn sparse_available() -> Result<(), String> {
    wsl_core::sparse_supported(ops::wsl_version().as_deref())
}

#[tauri::command]
async fn op_convert(st: State<'_, ops::OpState>, name: String, version: u8) -> Result<(), String> {
    ops::convert(&st, name, version)
}

#[tauri::command]
async fn op_sparse(st: State<'_, ops::OpState>, name: String, on: bool) -> Result<(), String> {
    ops::sparse(&st, name, on)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(CpuState::default())
        .manage(ops::OpState::default())
        .manage(pty::PtyState::default())
        .setup(|app| tray::setup(app).map_err(Into::into))
        .invoke_handler(tauri::generate_handler![
            distros,
            start,
            stop,
            delete_distro,
            set_default_distro,
            restart,
            terminal,
            shutdown,
            pty_open,
            pty_write,
            pty_resize,
            pty_close,
            layouts_load,
            project::project_parse,
            project::git_status,
            project::git_diff,
            project::fs_list,
            project::fs_read,
            store_load,
            detect_tools,
            store_save,
            layouts_save,
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
            open_external,
            read_wslconfig,
            write_wslconfig,
            read_distro_conf,
            write_distro_conf,
            config_load,
            config_save,
            op_default_base,
            op_set_default_base,
            op_status,
            op_cancel,
            op_export,
            op_import,
            op_clone,
            wsl_update_check,
            op_wsl_update,
            list_online,
            custom_install_available,
            op_create,
            sparse_available,
            op_convert,
            op_sparse
        ])
        .build(tauri::generate_context!())
        .expect("erreur au lancement de WSLCockpit")
        .run(|app, event| {
            // À la sortie, aucun terminal intégré ne doit survivre.
            if let tauri::RunEvent::Exit = event {
                app.state::<pty::PtyState>().kill_all();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses_web_autorisees() {
        assert!(
            is_web_url("https://localhost:17000/login?t=abc")
                && is_web_url("http://127.0.0.1:5000")
        );
        assert!(
            !is_web_url("file:///C:/Windows")
                && !is_web_url("javascript:alert(1)")
                && !is_web_url("ftp://x")
        );
        assert!(
            !is_web_url("https://a.b/x y")
                && !is_web_url("https://a.b/\"x")
                && !is_web_url("https://a.b/\nx")
        );
    }

    #[test]
    fn programmes_autorises() {
        assert!(allowed_program("wsl.exe") && allowed_program("PowerShell.exe"));
        assert!(!allowed_program("cmd.exe") && !allowed_program("C:\\x\\wsl.exe"));
    }
}
