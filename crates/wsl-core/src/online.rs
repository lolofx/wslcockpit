//! Distributions installables (`wsl --list --online`) et arguments d'installation.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineDistro {
    pub name: String,
    pub friendly: String,
}

/// Lit la sortie de `wsl --list --online` (UTF-16 déjà décodée). L'introduction et l'en-tête sont
/// localisés : on se fie à la structure (texte, ligne vide, en-tête, puis `NOM  NOM CONVIVIAL`).
pub fn parse_online(text: &str) -> Vec<OnlineDistro> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
    let Some(blank) = lines.iter().position(|l| l.trim().is_empty()) else {
        return Vec::new();
    };
    // Plusieurs lignes vides possibles avant le tableau : l'en-tête est la première ligne non vide ensuite.
    let mut it = lines[blank..].iter().filter(|l| !l.trim().is_empty());
    it.next(); // en-tête
    it.filter_map(|l| {
        let l = l.trim();
        let (name, rest) = l.split_once(char::is_whitespace).unwrap_or((l, ""));
        if name.is_empty() {
            return None;
        }
        let friendly = rest.trim();
        Some(OnlineDistro {
            name: name.to_string(),
            friendly: if friendly.is_empty() {
                name.to_string()
            } else {
                friendly.to_string()
            },
        })
    })
    .collect()
}

/// `--name` et `--location` demandent WSL 2.4.4 ou plus récent.
pub fn custom_install_supported(version: Option<&[u32]>) -> Result<(), String> {
    match version {
        Some(v) if v >= &[2, 4, 4][..] => Ok(()),
        Some(_) => Err(
            "Nom et dossier personnalisés demandent WSL 2.4.4 ou plus récent (wsl --update)."
                .into(),
        ),
        None => Err(
            "Version de WSL inconnue : nom et dossier personnalisés indisponibles (wsl --update)."
                .into(),
        ),
    }
}

pub fn install_args(distro: &str, name: Option<&str>, dir: Option<&str>) -> Vec<String> {
    let mut a: Vec<String> = ["--install", distro, "--no-launch"]
        .map(String::from)
        .to_vec();
    if let Some(n) = name {
        a.push("--name".into());
        a.push(n.into());
    }
    if let Some(d) = dir {
        a.push("--location".into());
        a.push(d.into());
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_en_francais() {
        let t = "Voici la liste des distributions valides pouvant être installées.\r\nInstallez à l’aide de la commande « wsl.exe --install <Distro> ».\r\n\r\nNOM                   NOM CONVIVIAL\r\nUbuntu                Ubuntu\r\nDebian                Debian GNU/Linux\r\nkali-linux            Kali Linux Rolling\r\n";
        let v = parse_online(t);
        assert_eq!(v.len(), 3);
        assert_eq!(
            v[1],
            OnlineDistro {
                name: "Debian".into(),
                friendly: "Debian GNU/Linux".into()
            }
        );
        assert_eq!(v[2].name, "kali-linux");
    }

    #[test]
    fn parse_vide_ou_sans_tableau() {
        assert!(parse_online("").is_empty());
        assert!(parse_online("erreur sans ligne vide").is_empty());
    }

    #[test]
    fn nom_sans_nom_convivial() {
        let v = parse_online("intro\n\nNAME  FRIENDLY NAME\nSolo\n");
        assert_eq!(v[0].friendly, "Solo");
    }

    #[test]
    fn arguments_installation() {
        assert_eq!(
            install_args("Ubuntu", None, None),
            ["--install", "Ubuntu", "--no-launch"]
        );
        let a = install_args("Ubuntu", Some("Dev"), Some("D:\\wsl\\dev"));
        assert_eq!(&a[3..], ["--name", "Dev", "--location", "D:\\wsl\\dev"]);
    }

    #[test]
    fn seuil_version() {
        assert!(custom_install_supported(Some(&[2, 4, 4])).is_ok());
        assert!(custom_install_supported(Some(&[2, 5, 0])).is_ok());
        assert!(custom_install_supported(Some(&[2, 4, 3])).is_err());
        assert!(custom_install_supported(None).is_err());
    }
}
