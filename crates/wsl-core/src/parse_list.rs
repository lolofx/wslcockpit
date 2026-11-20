/// État d'une distribution, indépendant de la langue de Windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistroState {
    Running,
    Stopped,
    Installing,
    /// Libellé non reconnu, conservé tel quel.
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistroEntry {
    pub name: String,
    pub state: DistroState,
    pub version: u8,
    pub is_default: bool,
}

/// Analyse la sortie décodée de `wsl -l -v`.
///
/// Indépendant de la langue : la ligne d'en-tête est ignorée, le nom est le premier mot, la
/// version le dernier, l'état ce qui est entre les deux. Les lignes illisibles sont ignorées.
pub fn parse_list(text: &str) -> Vec<DistroEntry> {
    text.lines()
        .map(|l| l.trim_matches(|c: char| c == '\u{feff}' || c == '\r'))
        .filter(|l| !l.trim().is_empty())
        .skip(1)
        .filter_map(parse_row)
        .collect()
}

fn parse_row(line: &str) -> Option<DistroEntry> {
    let trimmed = line.trim_start();
    let (is_default, rest) = match trimmed.strip_prefix('*') {
        Some(r) => (true, r),
        None => (false, trimmed),
    };
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }
    let version = tokens.last()?.parse::<u8>().ok()?;
    let name = tokens[0].to_string();
    let raw_state = tokens[1..tokens.len() - 1].join(" ");
    Some(DistroEntry {
        name,
        state: map_state(&raw_state),
        version,
        is_default,
    })
}

fn map_state(raw: &str) -> DistroState {
    let norm = raw.to_lowercase().replace('\u{2019}', "'");
    match norm.as_str() {
        "running" | "en cours d'exécution" => DistroState::Running,
        "stopped" | "arrêté" => DistroState::Stopped,
        "installing" | "installation en cours" => DistroState::Installing,
        _ => DistroState::Other(raw.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str = "  NAME            STATE           VERSION\r\n* Ubuntu          Running         2\r\n  Debian          Stopped         2\r\n  Old             Stopped         1\r\n";
    const FR: &str = "  NOM                    ÉTAT            VERSION\r\n* Ubuntu-24.04          En cours d’exécution    2\r\n  Debian                 Arrêté          2\r\n";

    #[test]
    fn liste_en() {
        let d = parse_list(EN);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].name, "Ubuntu");
        assert_eq!(d[0].state, DistroState::Running);
        assert!(d[0].is_default);
        assert_eq!(d[1].state, DistroState::Stopped);
        assert!(!d[1].is_default);
        assert_eq!(d[2].version, 1);
    }

    #[test]
    fn liste_fr_etats_accentues() {
        let d = parse_list(FR);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].name, "Ubuntu-24.04");
        assert_eq!(d[0].state, DistroState::Running);
        assert_eq!(d[1].state, DistroState::Stopped);
    }

    #[test]
    fn liste_vide_ou_en_tete_seul() {
        assert!(parse_list("").is_empty());
        assert!(parse_list("  NAME  STATE  VERSION\r\n").is_empty());
    }

    #[test]
    fn bom_residuel_en_tete() {
        let d = parse_list(&format!("\u{feff}{EN}"));
        assert_eq!(d.len(), 3);
    }

    #[test]
    fn etat_inconnu_conserve() {
        let d = parse_list("  NAME STATE VERSION\n  X          Converting      2\n");
        assert_eq!(d[0].state, DistroState::Other("Converting".into()));
    }

    #[test]
    fn ligne_illisible_ignoree() {
        let d = parse_list("  NAME STATE VERSION\n  garbage\n  Ok Stopped 2\n");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].name, "Ok");
    }
}
