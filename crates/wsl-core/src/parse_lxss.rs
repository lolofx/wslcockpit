/// Distribution lue dans `HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LxssEntry {
    pub guid: String,
    pub name: Option<String>,
    pub base_path: Option<String>,
}

/// Analyse la sortie de `reg query <clé Lxss> /s`.
pub fn parse_lxss(text: &str) -> Vec<LxssEntry> {
    let mut entries: Vec<LxssEntry> = Vec::new();
    for line in text.lines().map(str::trim_end) {
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            // Ligne de clé : seules les sous-clés `{guid}` nous intéressent.
            match line.rsplit('\\').next().filter(|k| k.starts_with('{')) {
                Some(guid) => entries.push(LxssEntry {
                    guid: guid.to_string(),
                    ..Default::default()
                }),
                None => entries.push(LxssEntry::default()),
            }
            continue;
        }
        let Some(current) = entries.last_mut() else {
            continue;
        };
        let (key, rest) = split_word(line.trim_start());
        let (kind, value) = split_word(rest);
        if kind != "REG_SZ" {
            continue;
        }
        let value = value.trim().to_string();
        match key {
            "DistributionName" => current.name = Some(value),
            "BasePath" => current.base_path = Some(value),
            _ => {}
        }
    }
    entries.retain(|e| !e.guid.is_empty());
    entries
}

fn split_word(s: &str) -> (&str, &str) {
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}

/// Chemin du disque virtuel d'une distribution, sans le préfixe `\\?\`.
pub fn vhdx_path(base_path: &str) -> String {
    let base = base_path.strip_prefix(r"\\?\").unwrap_or(base_path);
    format!("{}\\ext4.vhdx", base.trim_end_matches('\\'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REG: &str = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss\r\n    DefaultDistribution    REG_SZ    {aaa}\r\n\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss\\{aaa}\r\n    State    REG_DWORD    0x1\r\n    DistributionName    REG_SZ    Ubuntu\r\n    BasePath    REG_SZ    \\\\?\\C:\\Users\\moi\\AppData\\Local\\Packages\\Canonical\\LocalState\r\n    Version    REG_DWORD    0x2\r\n\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss\\{bbb}\r\n    DistributionName    REG_SZ    Debian\r\n    BasePath    REG_SZ    E:\\wsl\\mon debian\r\n";

    #[test]
    fn lit_les_distros_et_ignore_la_clef_racine() {
        let e = parse_lxss(REG);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].guid, "{aaa}");
        assert_eq!(e[0].name.as_deref(), Some("Ubuntu"));
        assert_eq!(e[1].name.as_deref(), Some("Debian"));
    }

    #[test]
    fn chemin_avec_espace_conserve() {
        let e = parse_lxss(REG);
        assert_eq!(e[1].base_path.as_deref(), Some("E:\\wsl\\mon debian"));
    }

    #[test]
    fn chemin_du_vhdx() {
        assert_eq!(
            vhdx_path(r"\\?\C:\x\LocalState"),
            r"C:\x\LocalState\ext4.vhdx"
        );
        assert_eq!(vhdx_path(r"E:\wsl\d\"), r"E:\wsl\d\ext4.vhdx");
    }

    #[test]
    fn sortie_vide() {
        assert!(parse_lxss("").is_empty());
    }
}
