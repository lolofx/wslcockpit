//! Projet : cibles (distro + dossier Linux, ou dossier Windows), commandes git et parseurs purs.
//! Aucune exécution ici : les commandes sont construites en tableau, jamais concaténées en chaîne shell.

/// Dossier d'un projet : dans une distro (chemin Linux) ou sur un disque Windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Root {
    /// Vide = dossier Windows.
    pub distro: String,
    pub path: String,
}

/// Lit « Debian:/home/me/proj », un chemin UNC `\\wsl.localhost\Debian\home\me`, ou un chemin Windows.
pub fn parse_root(text: &str) -> Option<Root> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    for prefix in [r"\\wsl.localhost\", r"\\wsl$\"] {
        if let Some(rest) = t.strip_prefix(prefix) {
            let (distro, tail) = rest.split_once('\\').unwrap_or((rest, ""));
            if distro.is_empty() {
                return None;
            }
            let path = format!("/{}", tail.replace('\\', "/"));
            return Some(Root {
                distro: distro.into(),
                path: path.trim_end_matches('/').to_string().max("/".to_string()),
            });
        }
    }
    let b = t.as_bytes();
    let windows =
        (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':') || t.starts_with(r"\\");
    if windows {
        return Some(Root {
            distro: String::new(),
            path: t.to_string(),
        });
    }
    let (distro, path) = t.split_once(':')?;
    if distro.is_empty() || !path.starts_with('/') {
        return None;
    }
    Some(Root {
        distro: distro.into(),
        path: path.to_string(),
    })
}

/// Chemin vu depuis Windows (UNC pour une distro).
pub fn windows_path(root: &Root, rel: &str) -> String {
    let rel = rel
        .trim_matches(|c| c == '/' || c == '\\')
        .replace('/', "\\");
    let base = if root.distro.is_empty() {
        root.path.trim_end_matches('\\').to_string()
    } else {
        format!(
            r"\\wsl.localhost\{}{}",
            root.distro,
            root.path.replace('/', "\\").trim_end_matches('\\')
        )
    };
    if rel.is_empty() {
        base
    } else {
        format!(r"{base}\{rel}")
    }
}

/// Un chemin relatif sans `..` ni racine : ne sort jamais du projet.
pub fn safe_rel(rel: &str) -> bool {
    !rel.starts_with('/')
        && !rel.starts_with('\\')
        && !rel.contains(':')
        && !rel.split(['/', '\\']).any(|p| p == "..")
}

/// Programme et arguments pour lancer `git <args>` dans le projet.
pub fn git_command(root: &Root, args: &[&str]) -> (String, Vec<String>) {
    if root.distro.is_empty() {
        let mut a = vec!["-C".to_string(), root.path.clone()];
        a.extend(args.iter().map(|s| s.to_string()));
        ("git.exe".into(), a)
    } else {
        let mut a: Vec<String> = ["-d", &root.distro, "--cd", &root.path, "-e", "git"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        a.extend(args.iter().map(|s| s.to_string()));
        ("wsl.exe".into(), a)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitFile {
    pub path: String,
    /// Colonne d'index (X) et de l'arbre de travail (Y) de `git status --porcelain`.
    pub x: char,
    pub y: char,
    /// Ancien nom pour un renommage.
    pub from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitStatus {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<GitFile>,
}

pub const STATUS_ARGS: [&str; 5] = [
    "status",
    "--porcelain=v1",
    "-b",
    "-z",
    "--untracked-files=all",
];

/// Parse la sortie de `git status --porcelain=v1 -b -z`.
pub fn parse_status(text: &str) -> GitStatus {
    let mut st = GitStatus::default();
    let mut recs = text.split('\0').filter(|r| !r.is_empty());
    while let Some(rec) = recs.next() {
        if let Some(h) = rec.strip_prefix("## ") {
            parse_branch(h, &mut st);
            continue;
        }
        let mut c = rec.chars();
        let (x, y) = (c.next().unwrap_or(' '), c.next().unwrap_or(' '));
        let path: String = rec.chars().skip(3).collect();
        let from = if x == 'R' || x == 'C' || y == 'R' || y == 'C' {
            recs.next().map(str::to_string)
        } else {
            None
        };
        st.files.push(GitFile { path, x, y, from });
    }
    st
}

fn parse_branch(h: &str, st: &mut GitStatus) {
    if let Some(b) = h.strip_prefix("No commits yet on ") {
        st.branch = b.to_string();
        return;
    }
    if h.starts_with("HEAD (no branch)") {
        st.branch = "HEAD détachée".into();
        return;
    }
    let (names, tracking) = match h.find(" [") {
        Some(i) => (&h[..i], Some(h[i + 2..].trim_end_matches(']'))),
        None => (h, None),
    };
    match names.split_once("...") {
        Some((b, u)) => {
            st.branch = b.into();
            st.upstream = Some(u.into());
        }
        None => st.branch = names.into(),
    }
    for part in tracking.unwrap_or("").split(", ") {
        if let Some(n) = part.strip_prefix("ahead ") {
            st.ahead = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            st.behind = n.parse().unwrap_or(0);
        }
    }
}

impl GitFile {
    pub fn staged(&self) -> bool {
        self.x != ' ' && self.x != '?'
    }
    pub fn changed(&self) -> bool {
        self.y != ' ' && self.x != '?'
    }
    pub fn untracked(&self) -> bool {
        self.x == '?'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn racines() {
        assert_eq!(
            parse_root("Debian:/home/me/p"),
            Some(Root {
                distro: "Debian".into(),
                path: "/home/me/p".into()
            })
        );
        assert_eq!(
            parse_root(r"\\wsl.localhost\Ubuntu\home\me\p\"),
            Some(Root {
                distro: "Ubuntu".into(),
                path: "/home/me/p".into()
            })
        );
        assert_eq!(
            parse_root(r"\\wsl.localhost\Ubuntu"),
            Some(Root {
                distro: "Ubuntu".into(),
                path: "/".into()
            })
        );
        assert_eq!(
            parse_root(r"C:\Dev\app"),
            Some(Root {
                distro: String::new(),
                path: r"C:\Dev\app".into()
            })
        );
        assert_eq!(parse_root("rien"), None);
        assert_eq!(parse_root(""), None);
        assert_eq!(
            parse_root("D:relatif/x").map(|r| r.distro),
            Some(String::new())
        );
    }

    #[test]
    fn chemins_windows_et_surete() {
        let w = Root {
            distro: "Debian".into(),
            path: "/home/me/p".into(),
        };
        assert_eq!(
            windows_path(&w, "src/a.md"),
            r"\\wsl.localhost\Debian\home\me\p\src\a.md"
        );
        assert_eq!(windows_path(&w, ""), r"\\wsl.localhost\Debian\home\me\p");
        let c = Root {
            distro: String::new(),
            path: r"C:\Dev\".into(),
        };
        assert_eq!(windows_path(&c, "a/b"), r"C:\Dev\a\b");
        assert!(safe_rel("src/a.md") && safe_rel(""));
        assert!(
            !safe_rel("../x")
                && !safe_rel("a/../../x")
                && !safe_rel("/etc")
                && !safe_rel(r"C:\x")
                && !safe_rel(r"..\x")
        );
    }

    #[test]
    fn commandes() {
        let w = Root {
            distro: "Debian".into(),
            path: "/home/me/p".into(),
        };
        let (p, a) = git_command(&w, &["status"]);
        assert_eq!(p, "wsl.exe");
        assert_eq!(
            a,
            ["-d", "Debian", "--cd", "/home/me/p", "-e", "git", "status"]
        );
        let c = Root {
            distro: String::new(),
            path: r"C:\Dev".into(),
        };
        let (p, a) = git_command(&c, &["diff", "--", "a b.md"]);
        assert_eq!(p, "git.exe");
        assert_eq!(a, ["-C", r"C:\Dev", "diff", "--", "a b.md"]);
    }

    #[test]
    fn statut() {
        let out = "## main...origin/main [ahead 2, behind 1]\0 M src/a.rs\0A  new file.md\0R  nouveau.txt\0ancien.txt\0?? tmp/x\0MM both\0";
        let s = parse_status(out);
        assert_eq!(
            (s.branch.as_str(), s.upstream.as_deref(), s.ahead, s.behind),
            ("main", Some("origin/main"), 2, 1)
        );
        assert_eq!(s.files.len(), 5);
        assert_eq!(
            s.files[0],
            GitFile {
                path: "src/a.rs".into(),
                x: ' ',
                y: 'M',
                from: None
            }
        );
        assert_eq!(s.files[1].path, "new file.md");
        assert!(s.files[1].staged() && !s.files[1].changed());
        assert_eq!(s.files[2].from.as_deref(), Some("ancien.txt"));
        assert!(s.files[3].untracked() && !s.files[3].staged() && !s.files[3].changed());
        assert!(s.files[4].staged() && s.files[4].changed());
    }

    #[test]
    fn branches_particulieres() {
        assert_eq!(parse_status("## No commits yet on main\0").branch, "main");
        assert_eq!(
            parse_status("## HEAD (no branch)\0").branch,
            "HEAD détachée"
        );
        let s = parse_status("## dev\0");
        assert_eq!((s.branch.as_str(), s.upstream), ("dev", None));
        assert_eq!(
            parse_status("## dev...o/dev [gone]\0").upstream.as_deref(),
            Some("o/dev")
        );
        assert!(parse_status("").files.is_empty());
    }

    #[test]
    fn sortie_reelle_de_git() {
        // capturée avec git 2.x : `git status --porcelain=v1 -b -z --untracked-files=all`
        let s = parse_status("## main\0 M a.txt\0R  d.txt\0b c.txt\0?? new.md\0");
        assert_eq!(s.branch, "main");
        assert_eq!(s.files.len(), 3);
        assert_eq!(
            (s.files[1].path.as_str(), s.files[1].from.as_deref()),
            ("d.txt", Some("b c.txt"))
        );
        assert_eq!(s.files[2].path, "new.md");
    }
}
