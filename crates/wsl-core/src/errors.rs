/// Erreur d'une commande `wsl.exe`, interprétée à partir du code de retour et du texte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WslError {
    /// WSL n'est pas installé ou pas activé.
    NotInstalled,
    DistroNotFound,
    /// Cause non reconnue : code de retour et texte conservés pour le détail technique.
    Other {
        code: i32,
        message: String,
    },
}

/// Classe l'échec d'une commande `wsl.exe` (sortie décodée, code de retour non nul).
pub fn classify_failure(code: i32, output: &str) -> WslError {
    let low = output.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| low.contains(n));
    if has(&[
        "not installed",
        "n'est pas installé",
        "n’est pas installé",
        "wsl_e_wsl_optional_component_required",
        "optional component",
    ]) {
        WslError::NotInstalled
    } else if has(&[
        "there is no distribution",
        "wsl_e_distro_not_found",
        "aucune distribution",
        "n'existe pas",
        "n’existe pas",
    ]) {
        WslError::DistroNotFound
    } else {
        WslError::Other {
            code,
            message: output.trim().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distro_introuvable_en() {
        let e = classify_failure(-1, "There is no distribution with the supplied name.\r\nError code: Wsl/Service/WSL_E_DISTRO_NOT_FOUND");
        assert_eq!(e, WslError::DistroNotFound);
    }

    #[test]
    fn wsl_absent() {
        let e = classify_failure(
            1,
            "Windows Subsystem for Linux has no installed distributions. WSL is not installed.",
        );
        assert_eq!(e, WslError::NotInstalled);
    }

    #[test]
    fn cause_inconnue_conserve_code_et_texte() {
        let e = classify_failure(42, "  boom \r\n");
        assert_eq!(
            e,
            WslError::Other {
                code: 42,
                message: "boom".into()
            }
        );
    }
}
