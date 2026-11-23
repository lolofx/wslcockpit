/// Fichier INI édité en place : chaque ligne non touchée est conservée telle quelle
/// (commentaires, clés inconnues, espaces, fins de ligne).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ini {
    lines: Vec<String>,
    eol: &'static str,
    final_eol: bool,
    touched: bool,
}

fn section_name(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[')?.strip_suffix(']').map(str::trim)
}

/// (début de la valeur dans la ligne, clé) si la ligne est de la forme `clé = valeur`.
fn key_value(line: &str) -> Option<(usize, &str)> {
    let t = line.trim_start();
    if t.is_empty() || t.starts_with('#') || t.starts_with(';') || t.starts_with('[') {
        return None;
    }
    let eq = line.find('=')?;
    Some((eq + 1, line[..eq].trim()))
}

impl Ini {
    pub fn parse(text: &str) -> Self {
        Self {
            eol: if text.contains("\r\n") { "\r\n" } else { "\n" },
            final_eol: text.ends_with('\n'),
            touched: false,
            lines: text.lines().map(str::to_string).collect(),
        }
    }

    /// Plage de lignes `[début, fin)` du contenu d'une section (hors en-tête).
    fn section_range(&self, section: &str) -> Option<(usize, usize)> {
        let start = self
            .lines
            .iter()
            .position(|l| section_name(l).is_some_and(|s| s.eq_ignore_ascii_case(section)))?
            + 1;
        let end = self.lines[start..]
            .iter()
            .position(|l| section_name(l).is_some())
            .map_or(self.lines.len(), |i| start + i);
        Some((start, end))
    }

    fn find_key(&self, section: &str, key: &str) -> Option<usize> {
        let (s, e) = self.section_range(section)?;
        (s..e)
            .find(|&i| key_value(&self.lines[i]).is_some_and(|(_, k)| k.eq_ignore_ascii_case(key)))
    }

    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        let i = self.find_key(section, key)?;
        let (pos, _) = key_value(&self.lines[i])?;
        Some(self.lines[i][pos..].trim().to_string())
    }

    /// Modifie la valeur d'une clé, ou l'ajoute (et la section au besoin).
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        if let Some(i) = self.find_key(section, key) {
            let (pos, _) = key_value(&self.lines[i]).expect("clé trouvée");
            let after = &self.lines[i][pos..];
            let lead = after.len() - after.trim_start().len();
            self.lines[i] = format!("{}{value}", &self.lines[i][..pos + lead]);
            self.touched = true;
            return;
        }
        self.touched = true;
        let entry = format!("{key}={value}");
        match self.section_range(section) {
            Some((s, e)) => {
                // Après la dernière ligne non vide de la section.
                let at = (s..e)
                    .rev()
                    .find(|&i| !self.lines[i].trim().is_empty())
                    .map_or(s, |i| i + 1);
                self.lines.insert(at, entry);
            }
            None => {
                if self.lines.last().is_some_and(|l| !l.trim().is_empty()) {
                    self.lines.push(String::new());
                }
                self.lines.push(format!("[{section}]"));
                self.lines.push(entry);
            }
        }
    }

    /// Retire une clé ; renvoie vrai si elle existait.
    pub fn remove(&mut self, section: &str, key: &str) -> bool {
        match self.find_key(section, key) {
            Some(i) => {
                self.touched = true;
                self.lines.remove(i);
                true
            }
            None => false,
        }
    }

    pub fn render(&self) -> String {
        let mut out = self.lines.join(self.eol);
        if (self.final_eol || self.touched) && !out.is_empty() {
            out.push_str(self.eol);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# réglages perso\r\n[wsl2]\r\nmemory = 8GB   \r\n; garde moi\r\nunknownKey=42\r\n\r\n[boot]\r\nsystemd=true\r\n";

    #[test]
    fn aller_retour_sans_modification() {
        assert_eq!(Ini::parse(SAMPLE).render(), SAMPLE);
        let lf = "[a]\nb=c\n";
        assert_eq!(Ini::parse(lf).render(), lf);
    }

    #[test]
    fn modifier_ne_change_que_la_cle_editee() {
        let mut i = Ini::parse(SAMPLE);
        i.set("wsl2", "memory", "12GB");
        assert_eq!(
            i.render(),
            "# réglages perso\r\n[wsl2]\r\nmemory = 12GB\r\n; garde moi\r\nunknownKey=42\r\n\r\n[boot]\r\nsystemd=true\r\n"
        );
    }

    #[test]
    fn lecture_insensible_a_la_casse() {
        let i = Ini::parse(SAMPLE);
        assert_eq!(i.get("WSL2", "Memory").as_deref(), Some("8GB"));
        assert_eq!(i.get("boot", "systemd").as_deref(), Some("true"));
        assert_eq!(i.get("boot", "absent"), None);
    }

    #[test]
    fn ajout_dans_section_existante_avant_la_ligne_vide() {
        let mut i = Ini::parse(SAMPLE);
        i.set("wsl2", "processors", "4");
        assert!(i
            .render()
            .contains("unknownKey=42\r\nprocessors=4\r\n\r\n[boot]"));
    }

    #[test]
    fn ajout_de_section_et_fichier_absent() {
        let mut i = Ini::parse("");
        i.set("wsl2", "memory", "4GB");
        assert_eq!(i.render(), "[wsl2]\nmemory=4GB\n");
        let mut j = Ini::parse("[boot]\nsystemd=true");
        j.set("user", "default", "loic");
        assert_eq!(j.render(), "[boot]\nsystemd=true\n\n[user]\ndefault=loic\n");
    }

    #[test]
    fn suppression() {
        let mut i = Ini::parse(SAMPLE);
        assert!(i.remove("wsl2", "unknownKey"));
        assert!(!i.remove("wsl2", "unknownKey"));
        assert!(!i.render().contains("unknownKey"));
        assert!(i.render().contains("; garde moi"));
    }

    #[test]
    fn commentaires_ne_sont_pas_des_cles() {
        let i = Ini::parse("[a]\n# x=1\n; y=2\nz=3\n");
        assert_eq!(i.get("a", "x"), None);
        assert_eq!(i.get("a", "z").as_deref(), Some("3"));
    }
}
