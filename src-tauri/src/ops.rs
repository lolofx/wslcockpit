//! Export, import et clone en tâche de fond : une seule opération à la fois, annulable, avec progression.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use wsl_core::{
    check_new_name, check_tar_target, check_target_dir, convert_args, custom_install_supported,
    export_args, import_args, install_args, list_distros, run_cancellable, sparse_args,
    sparse_supported, WslExe,
};

#[derive(Clone, Default, Serialize)]
pub struct Status {
    /// Une opération est en cours.
    pub active: bool,
    pub kind: String,
    pub stage: String,
    /// Fin sans erreur (message de succès).
    pub done: Option<String>,
    pub error: Option<String>,
    /// Octets déjà écrits (archive ou dossier d'installation).
    pub bytes: u64,
    #[serde(skip)]
    watch: Option<PathBuf>,
}

#[derive(Default)]
struct Shared {
    status: Mutex<Status>,
    cancel: AtomicBool,
}

#[derive(Clone, Default)]
pub struct OpState(Arc<Shared>);

fn size_of(p: &Path) -> u64 {
    match std::fs::metadata(p) {
        Ok(m) if m.is_file() => m.len(),
        Ok(m) if m.is_dir() => std::fs::read_dir(p)
            .map(|it| it.flatten().map(|e| size_of(&e.path())).sum())
            .unwrap_or(0),
        _ => 0,
    }
}

impl OpState {
    pub fn status(&self) -> Status {
        let mut s = self.0.status.lock().unwrap().clone();
        if let Some(w) = &s.watch {
            s.bytes = size_of(w);
        }
        s
    }

    pub fn cancel(&self) {
        self.0.cancel.store(true, Ordering::SeqCst);
    }

    fn begin(&self, kind: &str) -> Result<(), String> {
        let mut s = self.0.status.lock().unwrap();
        if s.active {
            return Err("Une opération est déjà en cours.".into());
        }
        self.0.cancel.store(false, Ordering::SeqCst);
        *s = Status {
            active: true,
            kind: kind.into(),
            ..Default::default()
        };
        Ok(())
    }

    fn stage(&self, stage: &str, watch: Option<PathBuf>) {
        let mut s = self.0.status.lock().unwrap();
        s.stage = stage.into();
        s.watch = watch;
        s.bytes = 0;
    }

    fn finish(&self, r: Result<String, String>) {
        let mut s = self.0.status.lock().unwrap();
        s.active = false;
        s.watch = None;
        match r {
            Ok(m) => s.done = Some(m),
            Err(e) => s.error = Some(e),
        }
    }

    fn run(&self, args: Vec<String>) -> Result<(), String> {
        run_cancellable(&args, &self.0.cancel).map_err(|e| match e {
            wsl_core::WslError::Other { code: -1, message } => message,
            other => crate::msg(other),
        })
    }
}

fn existing_names() -> Result<Vec<String>, String> {
    list_distros(&WslExe)
        .map(|v| v.into_iter().map(|d| d.name).collect())
        .map_err(crate::msg)
}

fn path_str(p: &Path) -> String {
    p.display().to_string()
}

/// Supprime la distribution à moitié importée (elle n'existait pas avant l'opération).
fn unregister_partial(st: &OpState, name: &str) {
    if existing_names().is_ok_and(|l| l.iter().any(|n| n == name)) {
        let _ = st.run(vec!["--unregister".into(), name.into()]);
    }
}

pub fn export(st: &OpState, name: String, tar: String) -> Result<(), String> {
    let tar = PathBuf::from(tar);
    check_tar_target(&tar)?;
    st.begin("export")?;
    let st = st.clone();
    std::thread::spawn(move || {
        st.stage("Export en cours…", Some(tar.clone()));
        let r = st.run(export_args(&name, &path_str(&tar)));
        if r.is_err() {
            let _ = std::fs::remove_file(&tar);
        }
        st.finish(r.map(|_| format!("Exporté vers {}", tar.display())));
    });
    Ok(())
}

pub fn import(st: &OpState, name: String, dir: String, tar: String) -> Result<(), String> {
    check_new_name(&name, &existing_names()?)?;
    let (dir, tar) = (PathBuf::from(dir), PathBuf::from(tar));
    check_target_dir(&dir)?;
    if !tar.is_file() {
        return Err("Archive introuvable.".into());
    }
    st.begin("import")?;
    let st = st.clone();
    std::thread::spawn(move || {
        st.stage("Import en cours…", Some(dir.clone()));
        let r = st.run(import_args(&name, &path_str(&dir), &path_str(&tar)));
        if r.is_err() {
            unregister_partial(&st, &name);
        }
        st.finish(r.map(|_| format!("« {name} » importée.")));
    });
    Ok(())
}

pub fn clone(st: &OpState, source: String, new_name: String, dir: String) -> Result<(), String> {
    check_new_name(&new_name, &existing_names()?)?;
    let dir = PathBuf::from(dir);
    check_target_dir(&dir)?;
    let parent = dir.parent().filter(|p| p.is_dir()).ok_or(
        "Le dossier parent du dossier choisi doit exister (l'archive temporaire y est écrite).",
    )?;
    let tmp = parent.join(format!("{new_name}.clone-tmp.tar"));
    check_tar_target(&tmp)?;
    st.begin("clone")?;
    let st = st.clone();
    std::thread::spawn(move || {
        st.stage("1/2 Export de la source…", Some(tmp.clone()));
        let mut r = st.run(export_args(&source, &path_str(&tmp)));
        if r.is_ok() {
            st.stage("2/2 Import de la copie…", Some(dir.clone()));
            r = st.run(import_args(&new_name, &path_str(&dir), &path_str(&tmp)));
            if r.is_err() {
                unregister_partial(&st, &new_name);
            }
        }
        let _ = std::fs::remove_file(&tmp);
        st.finish(r.map(|_| format!("« {source} » clonée en « {new_name} ».")));
    });
    Ok(())
}

/// Version de WSL installée (`wsl --version`), `None` si absente ou trop ancienne pour la donner.
pub fn wsl_version() -> Option<Vec<u32>> {
    wsl_core::run_checked(&WslExe, &["--version"])
        .ok()
        .and_then(|t| wsl_core::parse_wsl_version(&t))
}

fn require_stopped(name: &str) -> Result<wsl_core::DistroEntry, String> {
    let d = list_distros(&WslExe)
        .map_err(crate::msg)?
        .into_iter()
        .find(|d| d.name == name)
        .ok_or("Cette distribution n'existe plus.")?;
    if d.state != wsl_core::DistroState::Stopped {
        return Err("Arrête la distribution avant cette opération.".into());
    }
    Ok(d)
}

pub fn convert(st: &OpState, name: String, version: u8) -> Result<(), String> {
    if !matches!(version, 1 | 2) {
        return Err("Version WSL invalide (1 ou 2).".into());
    }
    let d = require_stopped(&name)?;
    if d.version == version {
        return Err(format!("« {name} » est déjà en WSL{version}."));
    }
    st.begin("convert")?;
    let st = st.clone();
    std::thread::spawn(move || {
        st.stage(&format!("Conversion en WSL{version}…"), None);
        let r = st.run(convert_args(&name, version));
        st.finish(r.map(|_| format!("« {name} » est maintenant en WSL{version}.")));
    });
    Ok(())
}

pub fn sparse(st: &OpState, name: String, on: bool) -> Result<(), String> {
    sparse_supported(wsl_version().as_deref())?;
    require_stopped(&name)?;
    st.begin("sparse")?;
    let st = st.clone();
    std::thread::spawn(move || {
        st.stage("Changement du mode sparse…", None);
        let r = st.run(sparse_args(&name, on));
        let msg = if on { "activé" } else { "désactivé" };
        st.finish(r.map(|_| format!("Mode sparse {msg} pour « {name} ».")));
    });
    Ok(())
}

/// Installe une distribution du catalogue en ligne. `name` et `dir` (facultatifs, WSL 2.4.4+)
/// donnent un nom et un dossier propres ; sans eux WSL choisit le nom du catalogue.
pub fn create(
    st: &OpState,
    distro: String,
    name: Option<String>,
    dir: Option<String>,
) -> Result<(), String> {
    let existing = existing_names()?;
    let final_name = name.clone().unwrap_or_else(|| distro.clone());
    check_new_name(&final_name, &existing)?;
    if name.is_some() || dir.is_some() {
        custom_install_supported(wsl_version().as_deref())?;
    }
    let dir = dir.map(PathBuf::from);
    if let Some(d) = &dir {
        check_target_dir(d)?;
    }
    st.begin("create")?;
    let st = st.clone();
    std::thread::spawn(move || {
        st.stage(
            &format!("Téléchargement et installation de {distro}…"),
            dir.clone(),
        );
        let args = install_args(
            &distro,
            name.as_deref(),
            dir.as_deref().map(path_str).as_deref(),
        );
        let r = st.run(args);
        if r.is_err() {
            unregister_partial(&st, &final_name);
        }
        st.finish(r.map(|_| {
            format!("« {final_name} » installée. Ouvre un terminal pour créer l'utilisateur.")
        }));
    });
    Ok(())
}
