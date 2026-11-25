//! Projet : git en lecture seule et lecture de fichiers dans un dossier de distro ou de Windows.
//! Rien n'écrit : status, diff, liste de dossier, lecture d'un fichier texte.

use serde::{Deserialize, Serialize};
use std::process::Command;
use wsl_core::{
    decode_output, git_command, parse_root, parse_status, safe_rel, windows_path, Root, STATUS_ARGS,
};

const MAX_READ: u64 = 1024 * 1024;

#[derive(Serialize, Deserialize, Clone)]
pub struct RootDto {
    pub distro: String,
    pub path: String,
}
impl From<RootDto> for Root {
    fn from(r: RootDto) -> Self {
        Root {
            distro: r.distro,
            path: r.path,
        }
    }
}

#[derive(Serialize)]
pub struct FileDto {
    path: String,
    x: char,
    y: char,
    from: Option<String>,
}

#[derive(Serialize)]
pub struct StatusDto {
    branch: String,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
    files: Vec<FileDto>,
}

#[derive(Serialize)]
pub struct EntryDto {
    name: String,
    dir: bool,
}

/// Lance git et renvoie (code, stdout, stderr). Erreur lisible si git ou wsl est introuvable.
fn run(root: &Root, args: &[&str]) -> Result<(i32, Vec<u8>, String), String> {
    let (program, a) = git_command(root, args);
    let mut cmd = Command::new(&program);
    cmd.args(&a);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound if program == "git.exe" => "GIT_NOT_FOUND".to_string(),
        std::io::ErrorKind::NotFound => "WSL_ABSENT".to_string(),
        _ => e.to_string(),
    })?;
    let err = decode_output(&out.stderr);
    let code = out.status.code().unwrap_or(-1);
    if code == 127
        || (code != 0
            && (err.contains("git: not found")
                || err.contains("git: command not found")
                || err.contains("'git' is not recognized")))
    {
        return Err("GIT_NOT_FOUND".into());
    }
    Ok((code, out.stdout, err))
}

/// Chemin d'accès aux fichiers : UNC / disque sous Windows ; ailleurs (tests) le chemin tel quel.
fn fs_path(root: &Root, rel: &str) -> String {
    if cfg!(windows) {
        windows_path(root, rel)
    } else {
        std::path::Path::new(&root.path)
            .join(rel)
            .to_string_lossy()
            .into_owned()
    }
}

fn checked(root: &Root, args: &[&str]) -> Result<Vec<u8>, String> {
    let (code, out, err) = run(root, args)?;
    if code == 0 {
        return Ok(out);
    }
    if err.contains("not a git repository") {
        return Err("NOT_REPO".into());
    }
    Err(format!("git a échoué ({code}) : {}", err.trim()))
}

#[tauri::command]
pub fn project_parse(text: String) -> Option<RootDto> {
    parse_root(&text).map(|r| RootDto {
        distro: r.distro,
        path: r.path,
    })
}

#[tauri::command]
pub fn git_status(root: RootDto) -> Result<StatusDto, String> {
    let root: Root = root.into();
    let out = checked(&root, &STATUS_ARGS)?;
    let s = parse_status(&String::from_utf8_lossy(&out));
    Ok(StatusDto {
        branch: s.branch,
        upstream: s.upstream,
        ahead: s.ahead,
        behind: s.behind,
        files: s
            .files
            .into_iter()
            .map(|f| FileDto {
                path: f.path,
                x: f.x,
                y: f.y,
                from: f.from,
            })
            .collect(),
    })
}

/// Diff d'un fichier suivi : de l'index (`staged`) ou de l'arbre de travail.
#[tauri::command]
pub fn git_diff(root: RootDto, path: String, staged: bool) -> Result<String, String> {
    if !safe_rel(&path) {
        return Err("Chemin refusé.".into());
    }
    let root: Root = root.into();
    let mut args = vec!["diff", "--no-color", "--no-ext-diff"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(&path);
    let out = checked(&root, &args)?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

#[tauri::command]
pub fn fs_list(root: RootDto, rel: String) -> Result<Vec<EntryDto>, String> {
    if !safe_rel(&rel) {
        return Err("Chemin refusé.".into());
    }
    let dir = fs_path(&root.into(), &rel);
    let mut out: Vec<EntryDto> = std::fs::read_dir(&dir)
        .map_err(|e| format!("Lecture de {dir} impossible : {e}"))?
        .filter_map(|e| e.ok())
        .map(|e| EntryDto {
            name: e.file_name().to_string_lossy().into_owned(),
            dir: e.file_type().map(|t| t.is_dir()).unwrap_or(false),
        })
        .filter(|e| e.name != ".git")
        .collect();
    out.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

#[tauri::command]
pub fn fs_read(root: RootDto, rel: String) -> Result<String, String> {
    if !safe_rel(&rel) || rel.is_empty() {
        return Err("Chemin refusé.".into());
    }
    let file = fs_path(&root.into(), &rel);
    let meta =
        std::fs::metadata(&file).map_err(|e| format!("Lecture de {file} impossible : {e}"))?;
    if meta.len() > MAX_READ {
        return Err("TOO_BIG".into());
    }
    let bytes = std::fs::read(&file).map_err(|e| format!("Lecture de {file} impossible : {e}"))?;
    if bytes.contains(&0) {
        return Err("BINARY".into());
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn liste_et_lecture_dans_un_dossier_local() {
        let d = std::env::temp_dir().join(format!("wslc-proj-{}", std::process::id()));
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::create_dir_all(d.join(".git")).unwrap();
        std::fs::write(d.join("a.md"), "# titre").unwrap();
        std::fs::write(d.join("bin.dat"), [1u8, 0, 2]).unwrap();
        // chemin « Windows » avec séparateurs Unix : valable pour le test local
        let root = RootDto {
            distro: String::new(),
            path: d.to_string_lossy().into_owned(),
        };
        let names: Vec<String> = fs_list(root.clone(), String::new())
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["sub", "a.md", "bin.dat"]);
        assert!(fs_list(root.clone(), "../x".into()).is_err());
        assert_eq!(fs_read(root.clone(), "a.md".into()).unwrap(), "# titre");
        assert_eq!(
            fs_read(root.clone(), "bin.dat".into()).unwrap_err(),
            "BINARY"
        );
        assert!(fs_read(root, "../etc/passwd".into()).is_err());
        std::fs::remove_dir_all(d).ok();
    }
}
