use crate::decode::decode_output;
use crate::errors::{classify_failure, WslError};
use crate::parse_list::{parse_list, DistroEntry};
use std::process::Command;

/// Sortie brute d'une commande.
pub struct RawOutput {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Exécute `wsl.exe` : séparé en trait pour simuler WSL dans les tests.
pub trait WslRunner {
    fn run(&self, args: &[&str]) -> Result<RawOutput, WslError>;
}

/// Lance le vrai `wsl.exe`. Les arguments sont passés en tableau, jamais concaténés.
pub struct WslExe;

impl WslRunner for WslExe {
    fn run(&self, args: &[&str]) -> Result<RawOutput, WslError> {
        let mut cmd = Command::new("wsl.exe");
        cmd.args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let out = cmd.output().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => WslError::NotInstalled,
            _ => WslError::Other {
                code: e.raw_os_error().unwrap_or(-1),
                message: e.to_string(),
            },
        })?;
        Ok(RawOutput {
            code: out.status.code().unwrap_or(-1),
            stdout: out.stdout,
            stderr: out.stderr,
        })
    }
}

/// Exécute une commande et renvoie sa sortie décodée, ou l'erreur classée.
pub fn run_checked(runner: &dyn WslRunner, args: &[&str]) -> Result<String, WslError> {
    let out = runner.run(args)?;
    if out.code == 0 {
        Ok(decode_output(&out.stdout))
    } else {
        let mut text = decode_output(&out.stdout);
        text.push_str(&decode_output(&out.stderr));
        Err(classify_failure(out.code, &text))
    }
}

pub fn list_distros(runner: &dyn WslRunner) -> Result<Vec<DistroEntry>, WslError> {
    run_checked(runner, &["-l", "-v"]).map(|t| parse_list(&t))
}

/// Arrête une distribution (`wsl -t`).
pub fn stop_distro(runner: &dyn WslRunner, name: &str) -> Result<(), WslError> {
    run_checked(runner, &["-t", name]).map(|_| ())
}

/// Arrête toutes les distributions et la VM (`wsl --shutdown`).
pub fn shutdown_all(runner: &dyn WslRunner) -> Result<(), WslError> {
    run_checked(runner, &["--shutdown"]).map(|_| ())
}

/// Démarre une distribution : lance un processus de maintien en vie sans attendre sa fin.
pub fn start_distro(name: &str) -> Result<(), WslError> {
    let mut cmd = Command::new("wsl.exe");
    cmd.args(["-d", name, "--exec", "sleep", "infinity"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => WslError::NotInstalled,
            _ => WslError::Other {
                code: e.raw_os_error().unwrap_or(-1),
                message: e.to_string(),
            },
        })
}

/// Ouvre un terminal sur la distribution : Windows Terminal, sinon une console classique.
pub fn open_terminal(name: &str) -> Result<(), WslError> {
    let other = |e: std::io::Error| WslError::Other {
        code: e.raw_os_error().unwrap_or(-1),
        message: e.to_string(),
    };
    let mut wt = Command::new("wt.exe");
    wt.args(["wsl.exe", "-d", name]);
    match wt.spawn() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut cmd = Command::new("wsl.exe");
            cmd.args(["-d", name]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0000_0010); // CREATE_NEW_CONSOLE
            }
            cmd.spawn().map(|_| ()).map_err(other)
        }
        Err(e) => Err(other(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_list::DistroState;

    struct Fake(RawOutput);

    impl WslRunner for Fake {
        fn run(&self, _args: &[&str]) -> Result<RawOutput, WslError> {
            Ok(RawOutput {
                code: self.0.code,
                stdout: self.0.stdout.clone(),
                stderr: self.0.stderr.clone(),
            })
        }
    }

    fn utf16(s: &str) -> Vec<u8> {
        let mut v = vec![0xFF, 0xFE];
        for u in s.encode_utf16() {
            v.extend_from_slice(&u.to_le_bytes());
        }
        v
    }

    #[test]
    fn liste_via_un_wsl_simule() {
        let f = Fake(RawOutput {
            code: 0,
            stdout: utf16("  NAME STATE VERSION\r\n* Ubuntu Running 2\r\n"),
            stderr: vec![],
        });
        let d = list_distros(&f).unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].state, DistroState::Running);
    }

    #[test]
    fn echec_classe_depuis_le_code_et_le_texte() {
        let f = Fake(RawOutput {
            code: 1,
            stdout: utf16("There is no distribution with the supplied name."),
            stderr: vec![],
        });
        assert_eq!(list_distros(&f), Err(WslError::DistroNotFound));
    }

    #[test]
    fn stop_passe_le_nom_en_argument_separe() {
        use std::cell::RefCell;
        struct Rec(RefCell<Vec<String>>);
        impl WslRunner for Rec {
            fn run(&self, args: &[&str]) -> Result<RawOutput, WslError> {
                *self.0.borrow_mut() = args.iter().map(|a| a.to_string()).collect();
                Ok(RawOutput {
                    code: 0,
                    stdout: vec![],
                    stderr: vec![],
                })
            }
        }
        let r = Rec(RefCell::new(vec![]));
        stop_distro(&r, "Ubuntu-24.04").unwrap();
        assert_eq!(*r.0.borrow(), vec!["-t", "Ubuntu-24.04"]);
        shutdown_all(&r).unwrap();
        assert_eq!(*r.0.borrow(), vec!["--shutdown"]);
    }
}
