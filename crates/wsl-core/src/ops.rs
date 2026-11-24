//! Opérations longues : export, import, clone. Contrôles purs, lancement annulable.

use crate::decode_output;
use crate::errors::{classify_failure, WslError};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub fn export_args(name: &str, tar: &str) -> Vec<String> {
    ["--export", name, tar].map(String::from).to_vec()
}

pub fn import_args(name: &str, dir: &str, tar: &str) -> Vec<String> {
    ["--import", name, dir, tar, "--version", "2"]
        .map(String::from)
        .to_vec()
}

pub fn convert_args(name: &str, version: u8) -> Vec<String> {
    ["--set-version", name, &version.to_string()]
        .map(String::from)
        .to_vec()
}

pub fn sparse_args(name: &str, on: bool) -> Vec<String> {
    [
        "--manage",
        name,
        "--set-sparse",
        if on { "true" } else { "false" },
    ]
    .map(String::from)
    .to_vec()
}

/// Le mode sparse (`wsl --manage --set-sparse`) demande WSL 2.0.0 ou plus récent.
pub fn sparse_supported(version: Option<&[u32]>) -> Result<(), String> {
    match version {
        Some(v) if v.first().copied().unwrap_or(0) >= 2 => Ok(()),
        Some(v) => Err(format!(
            "Mode sparse indisponible : WSL {} installé, 2.0.0 ou plus requis (wsl --update).",
            v.iter().map(u32::to_string).collect::<Vec<_>>().join(".")
        )),
        None => Err("Mode sparse indisponible : version de WSL inconnue (wsl --update).".into()),
    }
}

/// Nom d'une nouvelle distribution : lettres, chiffres, `_ - .`, 64 caractères max, inexistant.
pub fn check_new_name(name: &str, existing: &[String]) -> Result<(), String> {
    let n = name.trim();
    if n.is_empty() || n.len() > 64 || n.starts_with('-') {
        return Err("Nom invalide : 1 à 64 caractères, sans tiret au début.".into());
    }
    if !n
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err("Nom invalide : lettres, chiffres, _ - . seulement (pas d'espace).".into());
    }
    if existing.iter().any(|e| e.eq_ignore_ascii_case(n)) {
        return Err(format!("Une distribution « {n} » existe déjà."));
    }
    Ok(())
}

/// Dossier d'installation : absent, ou vide. Jamais un dossier qui contient déjà des fichiers.
pub fn check_target_dir(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    if !dir.is_dir() {
        return Err("Le chemin choisi est un fichier, pas un dossier.".into());
    }
    match std::fs::read_dir(dir) {
        Ok(mut it) => {
            if it.next().is_none() {
                Ok(())
            } else {
                Err("Le dossier existe déjà et n'est pas vide : choisis-en un autre.".into())
            }
        }
        Err(e) => Err(format!("Dossier illisible : {e}")),
    }
}

/// Archive d'export : extension `.tar`, ne remplace jamais un fichier existant.
pub fn check_tar_target(tar: &Path) -> Result<(), String> {
    if tar.extension().and_then(|e| e.to_str()) != Some("tar") {
        return Err("Le fichier d'export doit se terminer par .tar.".into());
    }
    if tar.exists() {
        return Err("Ce fichier existe déjà : il ne sera pas écrasé.".into());
    }
    match tar.parent() {
        Some(p) if p.as_os_str().is_empty() || p.is_dir() => Ok(()),
        _ => Err("Le dossier de destination n'existe pas.".into()),
    }
}

/// Lance `wsl.exe` et attend la fin ; tue le processus si `cancel` passe à vrai.
pub fn run_cancellable(args: &[String], cancel: &AtomicBool) -> Result<(), WslError> {
    let other = |e: std::io::Error| match e.kind() {
        std::io::ErrorKind::NotFound => WslError::NotInstalled,
        _ => WslError::Other {
            code: e.raw_os_error().unwrap_or(-1),
            message: e.to_string(),
        },
    };
    let mut cmd = Command::new("wsl.exe");
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(other)?;
    loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(WslError::Other {
                code: -1,
                message: "Opération annulée.".into(),
            });
        }
        match child.try_wait().map_err(other)? {
            Some(status) => {
                let out = child.wait_with_output().map_err(other)?;
                if status.success() {
                    return Ok(());
                }
                let mut text = decode_output(&out.stdout);
                text.push_str(&decode_output(&out.stderr));
                return Err(classify_failure(status.code().unwrap_or(-1), &text));
            }
            None => std::thread::sleep(Duration::from_millis(200)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        assert_eq!(
            export_args("U", r"C:\a b\u.tar"),
            ["--export", "U", r"C:\a b\u.tar"]
        );
        assert_eq!(
            import_args("N", r"D:\w", r"C:\u.tar"),
            ["--import", "N", r"D:\w", r"C:\u.tar", "--version", "2"]
        );
    }

    #[test]
    fn nom() {
        let ex = vec!["Ubuntu".to_string()];
        assert!(check_new_name("Ubuntu-clone", &ex).is_ok());
        assert!(check_new_name("ubuntu", &ex).is_err());
        assert!(check_new_name("", &ex).is_err());
        assert!(check_new_name("a b", &ex).is_err());
        assert!(check_new_name("-x", &ex).is_err());
        assert!(check_new_name(&"a".repeat(65), &ex).is_err());
    }

    #[test]
    fn dossier_existant_refuse_si_non_vide() {
        let base = std::env::temp_dir().join(format!("wslcockpit-ops-{}", std::process::id()));
        let vide = base.join("vide");
        std::fs::create_dir_all(&vide).unwrap();
        assert!(check_target_dir(&vide).is_ok());
        assert!(check_target_dir(&base.join("absent")).is_ok());
        std::fs::write(vide.join("f"), "x").unwrap();
        assert!(check_target_dir(&vide).is_err());
        assert!(check_target_dir(&vide.join("f")).is_err());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn archive_jamais_ecrasee() {
        let base = std::env::temp_dir().join(format!("wslcockpit-tar-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let t = base.join("a.tar");
        assert!(check_tar_target(&t).is_ok());
        std::fs::write(&t, "x").unwrap();
        assert!(check_tar_target(&t).is_err());
        assert!(check_tar_target(&base.join("a.zip")).is_err());
        assert!(check_tar_target(&base.join("nope").join("b.tar")).is_err());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn conversion_et_sparse() {
        assert_eq!(convert_args("U", 1), ["--set-version", "U", "1"]);
        assert_eq!(
            sparse_args("U", true),
            ["--manage", "U", "--set-sparse", "true"]
        );
        assert_eq!(sparse_args("U", false)[3], "false");
    }

    #[test]
    fn sparse_grise_si_wsl_ancien() {
        assert!(sparse_supported(Some(&[2, 3, 26, 0])).is_ok());
        assert!(sparse_supported(Some(&[2, 0, 0])).is_ok());
        let e = sparse_supported(Some(&[1, 2, 5])).unwrap_err();
        assert!(e.contains("1.2.5") && e.contains("2.0.0"));
        assert!(sparse_supported(None).is_err());
    }
}
