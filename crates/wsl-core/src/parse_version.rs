/// Extrait le numéro de version de WSL de la sortie décodée de `wsl --version`.
///
/// Première suite de chiffres séparés par des points trouvée (« WSL version: 2.3.26.0 » ou
/// « Version WSL : 2.3.26.0 »), pour ne pas dépendre de la langue.
pub fn parse_wsl_version(text: &str) -> Option<Vec<u32>> {
    let line = text
        .lines()
        .find(|l| l.chars().any(|c| c.is_ascii_digit()))?;
    line.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|w| w.contains('.'))
        .find_map(|w| {
            let parts: Option<Vec<u32>> = w
                .trim_matches('.')
                .split('.')
                .map(|p| p.parse().ok())
                .collect();
            parts.filter(|p| p.len() >= 2)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_en() {
        assert_eq!(
            parse_wsl_version("WSL version: 2.3.26.0\r\nKernel version: 5.15.167.4-1\r\n"),
            Some(vec![2, 3, 26, 0])
        );
    }

    #[test]
    fn version_fr() {
        assert_eq!(
            parse_wsl_version("Version WSL : 2.1.5.0\r\n"),
            Some(vec![2, 1, 5, 0])
        );
    }

    #[test]
    fn aucune_version() {
        assert_eq!(parse_wsl_version(""), None);
        assert_eq!(parse_wsl_version("Usage : wsl.exe\n"), None);
    }

    #[test]
    fn comparaison_de_versions() {
        assert!(parse_wsl_version("WSL version: 2.3.26.0").unwrap() >= vec![2, 3]);
    }
}
