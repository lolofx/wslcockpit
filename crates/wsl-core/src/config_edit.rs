//! Lecture et application de champs de config sur un texte INI, avec validation bloquante.

use crate::config_validate::{validate, Field};
use crate::ini::Ini;

/// Champs du `.wslconfig` global.
pub const GLOBAL_FIELDS: [Field; 5] = [
    Field::Memory,
    Field::Processors,
    Field::Swap,
    Field::NetworkingMode,
    Field::VmIdleTimeout,
];

/// Champs du `wsl.conf` d'une distro.
pub const DISTRO_FIELDS: [Field; 3] = [Field::Systemd, Field::DefaultUser, Field::AutomountEnabled];

impl Field {
    /// Identifiant stable utilisé par l'interface.
    pub fn id(self) -> &'static str {
        match self {
            Field::Memory => "memory",
            Field::Processors => "processors",
            Field::Swap => "swap",
            Field::NetworkingMode => "networkingMode",
            Field::VmIdleTimeout => "vmIdleTimeout",
            Field::Systemd => "systemd",
            Field::DefaultUser => "defaultUser",
            Field::AutomountEnabled => "automountEnabled",
        }
    }

    pub fn from_id(id: &str) -> Option<Field> {
        GLOBAL_FIELDS
            .iter()
            .chain(DISTRO_FIELDS.iter())
            .copied()
            .find(|f| f.id() == id)
    }
}

/// Valeurs actuelles (chaîne vide = clé absente) des champs donnés.
pub fn read_values(text: &str, fields: &[Field]) -> Vec<(String, String)> {
    let ini = Ini::parse(text);
    fields
        .iter()
        .map(|f| {
            let (s, k) = f.location();
            (f.id().to_string(), ini.get(s, k).unwrap_or_default())
        })
        .collect()
}

/// Applique des valeurs (vide = retirer la clé). Si une valeur est invalide, rien n'est écrit :
/// renvoie la liste (id, message) de toutes les erreurs.
pub fn apply_values(
    text: &str,
    allowed: &[Field],
    values: &[(String, String)],
) -> Result<String, Vec<(String, String)>> {
    let mut errors = Vec::new();
    let mut edits = Vec::new();
    for (id, v) in values {
        match Field::from_id(id).filter(|f| allowed.contains(f)) {
            None => errors.push((id.clone(), "Champ inconnu.".to_string())),
            Some(f) => {
                let v = v.trim();
                if !v.is_empty() {
                    if let Err(m) = validate(f, v) {
                        errors.push((id.clone(), m));
                        continue;
                    }
                }
                edits.push((f, v.to_string()));
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut ini = Ini::parse(text);
    for (f, v) in edits {
        let (s, k) = f.location();
        if v.is_empty() {
            ini.remove(s, k);
        } else {
            ini.set(s, k, &v);
        }
    }
    Ok(ini.render())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(a: &str, b: &str) -> (String, String) {
        (a.into(), b.into())
    }

    #[test]
    fn lit_les_valeurs_et_vide_si_absent() {
        let v = read_values("[wsl2]\nmemory=8GB\n", &GLOBAL_FIELDS);
        assert_eq!(v[0], kv("memory", "8GB"));
        assert_eq!(v[1], kv("processors", ""));
    }

    #[test]
    fn valeur_invalide_refuse_tout() {
        let src = "[wsl2]\nmemory=8GB\n";
        let r = apply_values(
            src,
            &GLOBAL_FIELDS,
            &[kv("memory", "16GB"), kv("processors", "0")],
        );
        let e = r.unwrap_err();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].0, "processors");
    }

    #[test]
    fn conserve_commentaires_et_cles_inconnues() {
        let src = "# note\n[wsl2]\nmemory=8GB\nautoMemoryReclaim=gradual\n";
        let out = apply_values(src, &GLOBAL_FIELDS, &[kv("memory", "16GB")]).unwrap();
        assert_eq!(
            out,
            "# note\n[wsl2]\nmemory=16GB\nautoMemoryReclaim=gradual\n"
        );
    }

    #[test]
    fn vide_retire_la_cle_et_fichier_absent_se_cree() {
        let out = apply_values("[wsl2]\nswap=2GB\n", &GLOBAL_FIELDS, &[kv("swap", "")]).unwrap();
        assert!(!out.contains("swap"));
        let neuf = apply_values("", &DISTRO_FIELDS, &[kv("systemd", "true")]).unwrap();
        assert!(neuf.contains("[boot]") && neuf.contains("systemd=true"));
    }

    #[test]
    fn refuse_un_champ_hors_cible() {
        let r = apply_values("", &DISTRO_FIELDS, &[kv("memory", "8GB")]);
        assert!(r.is_err());
    }
}
