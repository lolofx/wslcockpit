use std::collections::BTreeMap;
use std::path::PathBuf;

/// Alias locaux (nom réel -> alias). N'agit jamais sur WSL.
pub type Aliases = BTreeMap<String, String>;

pub fn settings_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("APPDATA").ok_or("Dossier %APPDATA% introuvable.")?;
    Ok(PathBuf::from(base).join("WSLCockpit").join("aliases.json"))
}

pub fn parse(text: &str) -> Aliases {
    serde_json::from_str(text).unwrap_or_default()
}

pub fn apply(mut a: Aliases, name: &str, alias: &str) -> Aliases {
    let alias = alias.trim();
    if alias.is_empty() {
        a.remove(name);
    } else {
        a.insert(name.to_string(), alias.to_string());
    }
    a
}

pub fn load() -> Aliases {
    settings_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| parse(&t))
        .unwrap_or_default()
}

pub fn save(a: &Aliases) -> Result<(), String> {
    let p = settings_path()?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(a).map_err(|e| e.to_string())?;
    std::fs::write(p, json).map_err(|e| format!("Écriture des réglages impossible : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ajoute_puis_retire() {
        let a = apply(Aliases::new(), "Ubuntu", "  Travail ");
        assert_eq!(a.get("Ubuntu").map(String::as_str), Some("Travail"));
        let a = apply(a, "Ubuntu", "   ");
        assert!(a.is_empty());
    }

    #[test]
    fn fichier_corrompu_donne_vide() {
        assert!(parse("pas du json").is_empty());
        assert_eq!(parse(r#"{"U":"x"}"#).len(), 1);
    }
}
