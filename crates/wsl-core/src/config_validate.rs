/// Champs de configuration connus et leur contrôle. Messages en français, affichés tels quels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    // .wslconfig, section [wsl2]
    Memory,
    Processors,
    Swap,
    NetworkingMode,
    VmIdleTimeout,
    // wsl.conf
    Systemd,
    DefaultUser,
    AutomountEnabled,
}

impl Field {
    /// (section, clé) dans le fichier.
    pub fn location(self) -> (&'static str, &'static str) {
        match self {
            Field::Memory => ("wsl2", "memory"),
            Field::Processors => ("wsl2", "processors"),
            Field::Swap => ("wsl2", "swap"),
            Field::NetworkingMode => ("wsl2", "networkingMode"),
            Field::VmIdleTimeout => ("wsl2", "vmIdleTimeout"),
            Field::Systemd => ("boot", "systemd"),
            Field::DefaultUser => ("user", "default"),
            Field::AutomountEnabled => ("automount", "enabled"),
        }
    }
}

fn size(v: &str) -> bool {
    // 8GB, 512MB, 0 ; unité obligatoire sauf pour 0.
    if v == "0" {
        return true;
    }
    let digits = v.chars().take_while(char::is_ascii_digit).count();
    let unit = &v[digits..];
    digits > 0
        && v[..digits].parse::<u64>().is_ok_and(|n| n > 0)
        && matches!(
            unit.to_ascii_uppercase().as_str(),
            "KB" | "MB" | "GB" | "TB" | "B"
        )
}

/// Valide une valeur ; `Ok` si elle peut être écrite telle quelle.
pub fn validate(field: Field, value: &str) -> Result<(), String> {
    let v = value.trim();
    let ok = match field {
        Field::Memory | Field::Swap => size(v),
        Field::Processors => v.parse::<u32>().is_ok_and(|n| n >= 1),
        Field::NetworkingMode => {
            matches!(
                v.to_ascii_lowercase().as_str(),
                "nat" | "mirrored" | "bridged" | "virtioproxy" | "none"
            )
        }
        Field::VmIdleTimeout => v.parse::<i64>().is_ok_and(|n| n >= -1),
        Field::Systemd | Field::AutomountEnabled => matches!(v, "true" | "false"),
        Field::DefaultUser => {
            !v.is_empty()
                && v.len() <= 32
                && !v.starts_with('-')
                && v.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        }
    };
    if ok {
        return Ok(());
    }
    Err(match field {
        Field::Memory => "Mémoire invalide : indique par exemple 8GB ou 4096MB.",
        Field::Swap => "Swap invalide : indique par exemple 4GB, ou 0 pour le désactiver.",
        Field::Processors => "Processeurs invalide : un nombre entier supérieur ou égal à 1.",
        Field::NetworkingMode => {
            "Mode réseau invalide : nat, mirrored, bridged, virtioproxy ou none."
        }
        Field::VmIdleTimeout => {
            "Délai d'inactivité invalide : un nombre de millisecondes (-1 pour désactiver)."
        }
        Field::Systemd => "systemd : true ou false.",
        Field::AutomountEnabled => "Montage automatique : true ou false.",
        Field::DefaultUser => "Utilisateur invalide : lettres, chiffres, _ - . seulement.",
    }
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailles() {
        for ok in ["8GB", "512mb", "0", "4096MB"] {
            assert!(validate(Field::Memory, ok).is_ok(), "{ok}");
        }
        for ko in ["", "8", "GB", "-1GB", "8 GB", "8GBB", "0GB"] {
            assert!(validate(Field::Memory, ko).is_err(), "{ko}");
        }
    }

    #[test]
    fn processeurs_reseau_delai() {
        assert!(validate(Field::Processors, "4").is_ok());
        assert!(validate(Field::Processors, "0").is_err());
        assert!(validate(Field::Processors, "x").is_err());
        assert!(validate(Field::NetworkingMode, "Mirrored").is_ok());
        assert!(validate(Field::NetworkingMode, "wifi").is_err());
        assert!(validate(Field::VmIdleTimeout, "-1").is_ok());
        assert!(validate(Field::VmIdleTimeout, "60000").is_ok());
        assert!(validate(Field::VmIdleTimeout, "-5").is_err());
    }

    #[test]
    fn booleens_et_utilisateur() {
        assert!(validate(Field::Systemd, "true").is_ok());
        assert!(validate(Field::Systemd, "yes").is_err());
        assert!(validate(Field::DefaultUser, "loic").is_ok());
        assert!(validate(Field::DefaultUser, "a b").is_err());
        assert!(validate(Field::DefaultUser, "-x").is_err());
        assert!(validate(Field::DefaultUser, "x\nsystemd=true").is_err());
    }

    #[test]
    fn emplacements() {
        assert_eq!(Field::Systemd.location(), ("boot", "systemd"));
        assert_eq!(Field::Memory.location(), ("wsl2", "memory"));
    }
}
