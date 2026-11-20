/// Décode la sortie brute de `wsl.exe`.
///
/// Les commandes de gestion (`-l`, `--status`, `--version`) écrivent en UTF-16 LE, avec ou sans
/// BOM ; la sortie d'une commande lancée dans la distro est de l'UTF-8.
pub fn decode_output(bytes: &[u8]) -> String {
    if looks_like_utf16le(bytes) {
        decode_utf16le(bytes)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn looks_like_utf16le(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return true;
    }
    // Sans BOM : du texte latin en UTF-16 LE a un octet nul sur chaque octet impair.
    let pairs = bytes.len() / 2;
    if pairs == 0 {
        return false;
    }
    let nul_high = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .filter(|c| c[1] == 0)
        .count();
    nul_high * 2 >= pairs
}

fn decode_utf16le(bytes: &[u8]) -> String {
    let body = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
    let units: Vec<u16> = body
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn utf16le(s: &str, bom: bool) -> Vec<u8> {
        let mut out = if bom { vec![0xFF, 0xFE] } else { vec![] };
        for u in s.encode_utf16() {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out
    }

    #[test]
    fn decode_utf16_avec_bom() {
        assert_eq!(decode_output(&utf16le("Ubuntu\r\n", true)), "Ubuntu\r\n");
    }

    #[test]
    fn decode_utf16_sans_bom() {
        assert_eq!(decode_output(&utf16le("Arrêté", false)), "Arrêté");
    }

    #[test]
    fn decode_utf8_de_la_distro() {
        assert_eq!(decode_output("été\n".as_bytes()), "été\n");
    }

    #[test]
    fn decode_vide() {
        assert_eq!(decode_output(&[]), "");
    }

    #[test]
    fn decode_octet_orphelin_ne_panique_pas() {
        let mut b = utf16le("ab", true);
        b.push(0x41);
        assert_eq!(decode_output(&b), "ab");
    }
}
