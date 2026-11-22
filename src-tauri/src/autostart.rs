use std::process::Command;

const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "WSLCockpit";

fn reg(args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut cmd = Command::new("reg.exe");
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.output()
}

pub fn is_enabled() -> bool {
    reg(&["query", KEY, "/v", VALUE])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Active ou retire le démarrage avec Windows (clé utilisateur, sans droits admin).
pub fn set_enabled(on: bool) -> Result<(), String> {
    let out = if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let data = format!("\"{}\"", exe.display());
        reg(&["add", KEY, "/v", VALUE, "/t", "REG_SZ", "/d", &data, "/f"])
    } else {
        reg(&["delete", KEY, "/v", VALUE, "/f"])
    }
    .map_err(|e| e.to_string())?;
    if out.status.success() || !on {
        Ok(())
    } else {
        Err("Impossible de modifier le démarrage avec Windows.".into())
    }
}
