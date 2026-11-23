//! Lecture et écriture sûres de `.wslconfig` et `wsl.conf` : toujours une sauvegarde horodatée avant d'écrire.

use crate::errors::WslError;
use crate::exec::{run_checked, WslRunner};
use crate::parse_list::{DistroEntry, DistroState};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `AAAAMMJJ-HHMMSS` (UTC) à partir de secondes Unix.
pub fn timestamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Jour civil depuis l'époque (algorithme de H. Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

pub fn now_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    timestamp(secs)
}

/// Nom du fichier de sauvegarde : `<fichier>.bak-<horodatage>`.
pub fn backup_name(file_name: &str, ts: &str) -> String {
    format!("{file_name}.bak-{ts}")
}

/// Écrit `text` dans `path` après avoir copié l'ancien fichier à côté. Renvoie la sauvegarde si elle existe.
/// Si la sauvegarde échoue, rien n'est écrit.
pub fn write_with_backup(path: &Path, text: &str, ts: &str) -> std::io::Result<Option<PathBuf>> {
    let mut backup = None;
    if path.exists() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("config");
        let b = path.with_file_name(backup_name(name, ts));
        std::fs::copy(path, &b)?;
        backup = Some(b);
    }
    std::fs::write(path, text)?;
    Ok(backup)
}

pub fn wslconf_needs_start(entries: &[DistroEntry], distro: &str) -> bool {
    !entries
        .iter()
        .any(|d| d.name == distro && d.state == DistroState::Running)
}

const WSLCONF: &str = "/etc/wsl.conf";

pub fn wslconf_read_args(distro: &str) -> Vec<String> {
    let script = format!("cat {WSLCONF} 2>/dev/null; true");
    ["-d", distro, "-u", "root", "--exec", "sh", "-c", &script]
        .map(String::from)
        .to_vec()
}

pub fn wslconf_write_script(ts: &str) -> String {
    format!(
        "if [ -f {WSLCONF} ]; then cp -p {WSLCONF} {WSLCONF}.bak-{ts} || exit 1; fi; cat > {WSLCONF}"
    )
}

pub fn read_wslconf(runner: &dyn WslRunner, distro: &str) -> Result<String, WslError> {
    let args = wslconf_read_args(distro);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_checked(runner, &refs)
}

/// Écrit `/etc/wsl.conf` dans la distro (sauvegarde `.bak-<ts>` d'abord), contenu transmis par stdin, fins de ligne LF.
pub fn write_wslconf(distro: &str, text: &str, ts: &str) -> Result<(), WslError> {
    let other = |e: std::io::Error| WslError::Other {
        code: e.raw_os_error().unwrap_or(-1),
        message: e.to_string(),
    };
    let mut cmd = Command::new("wsl.exe");
    cmd.args(["-d", distro, "-u", "root", "--exec", "sh", "-c"])
        .arg(wslconf_write_script(ts));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(other)?;
    let body = text.replace("\r\n", "\n");
    child
        .stdin
        .take()
        .ok_or_else(|| WslError::Other {
            code: -1,
            message: "stdin indisponible".into(),
        })?
        .write_all(body.as_bytes())
        .map_err(other)?;
    let out = child.wait_with_output().map_err(other)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(WslError::Other {
            code: out.status.code().unwrap_or(-1),
            message: format!(
                "{}{}",
                crate::decode_output(&out.stdout),
                crate::decode_output(&out.stderr)
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::RawOutput;

    #[test]
    fn timestamp_epoch_et_date_connue() {
        assert_eq!(timestamp(0), "19700101-000000");
        assert_eq!(timestamp(1_700_000_000), "20231114-221320");
        assert_eq!(timestamp(951_782_400 + 86_399), "20000229-235959");
    }

    #[test]
    fn nom_de_sauvegarde() {
        assert_eq!(
            backup_name(".wslconfig", "20260101-010203"),
            ".wslconfig.bak-20260101-010203"
        );
    }

    #[test]
    fn sauvegarde_avant_ecriture() {
        let dir = std::env::temp_dir().join(format!("wslcockpit-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join(".wslconfig");
        std::fs::write(&f, "ancien").unwrap();
        let b = write_with_backup(&f, "nouveau", "20260101-000000").unwrap();
        let b = b.expect("sauvegarde attendue");
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "ancien");
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "nouveau");
        // Premier enregistrement : pas d'ancien fichier, pas de sauvegarde.
        let g = dir.join("neuf");
        assert!(write_with_backup(&g, "x", "t").unwrap().is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn echec_de_sauvegarde_n_ecrit_rien() {
        let dir = std::env::temp_dir().join(format!("wslcockpit-test2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("c");
        std::fs::write(&f, "ancien").unwrap();
        // Une sauvegarde du même nom existe en tant que dossier : la copie échoue.
        std::fs::create_dir(dir.join("c.bak-t")).unwrap();
        assert!(write_with_backup(&f, "nouveau", "t").is_err());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "ancien");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn script_ecriture_sauvegarde_puis_ecrit() {
        let s = wslconf_write_script("T");
        assert!(s.contains("cp -p /etc/wsl.conf /etc/wsl.conf.bak-T || exit 1"));
        assert!(s.ends_with("cat > /etc/wsl.conf"));
    }

    fn entry(name: &str, state: DistroState) -> DistroEntry {
        DistroEntry {
            name: name.into(),
            state,
            version: 2,
            is_default: false,
        }
    }

    #[test]
    fn demarrage_requis_si_arretee() {
        let l = [
            entry("A", DistroState::Running),
            entry("B", DistroState::Stopped),
        ];
        assert!(!wslconf_needs_start(&l, "A"));
        assert!(wslconf_needs_start(&l, "B"));
        assert!(wslconf_needs_start(&l, "inconnue"));
    }

    struct Fake;
    impl WslRunner for Fake {
        fn run(&self, args: &[&str]) -> Result<RawOutput, WslError> {
            assert_eq!(&args[..5], ["-d", "Ubuntu", "-u", "root", "--exec"]);
            Ok(RawOutput {
                code: 0,
                stdout: b"[boot]\nsystemd=true\n".to_vec(),
                stderr: vec![],
            })
        }
    }

    #[test]
    fn lecture_wslconf() {
        assert_eq!(
            read_wslconf(&Fake, "Ubuntu").unwrap(),
            "[boot]\nsystemd=true\n"
        );
    }
}
