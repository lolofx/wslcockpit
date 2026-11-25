//! Terminaux intégrés : une session = un pseudo-terminal (ConPTY sous Windows) pilotant un processus.
//! Les sessions vivent côté Rust, indépendantes de l'affichage : déplacer un bloc, le détacher dans une
//! fenêtre ou changer de distribution ne coupe pas le processus. Les dernières sorties sont gardées
//! (anneau de `BUF_CAP` octets) pour être rejouées quand un affichage se rattache.
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const BUF_CAP: usize = 256 * 1024;

/// Anneau d'octets : garde les `cap` derniers, compte le total écrit (pour dédoublonner rejeu et direct).
pub struct Ring {
    buf: VecDeque<u8>,
    cap: usize,
    total: u64,
}

impl Ring {
    pub fn new(cap: usize) -> Self {
        Ring {
            buf: VecDeque::new(),
            cap,
            total: 0,
        }
    }
    /// Ajoute et renvoie le total cumulé après ajout.
    pub fn push(&mut self, bytes: &[u8]) -> u64 {
        self.buf.extend(bytes);
        while self.buf.len() > self.cap {
            self.buf.pop_front();
        }
        self.total += bytes.len() as u64;
        self.total
    }
    pub fn snapshot(&self) -> (Vec<u8>, u64) {
        (self.buf.iter().copied().collect(), self.total)
    }
}

pub struct Spec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub cols: u16,
    pub rows: u16,
    /// Variables ajoutées à l'environnement (liste blanche côté commande Tauri).
    pub env: Vec<(String, String)>,
}

/// WSLENV avec `name` ajouté : les variables listées passent de Windows vers la distro (wsl.exe).
pub fn wslenv_with(existing: Option<&str>, name: &str) -> String {
    match existing.filter(|e| !e.is_empty()) {
        Some(e) if e.split(':').any(|p| p.split('/').next() == Some(name)) => e.to_string(),
        Some(e) => format!("{e}:{name}"),
        None => name.to_string(),
    }
}

pub enum Event {
    /// `end` = total d'octets émis par la session une fois ce bloc inclus.
    Data {
        id: String,
        bytes: Vec<u8>,
        end: u64,
    },
    Exit {
        id: String,
    },
}

pub type Sink = Arc<dyn Fn(Event) + Send + Sync>;

pub struct Opened {
    pub replay: Vec<u8>,
    pub end: u64,
    pub alive: bool,
}

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    ring: Arc<Mutex<Ring>>,
    alive: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct PtyState {
    sessions: Mutex<HashMap<String, Session>>,
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}

impl PtyState {
    /// Ouvre la session `id`, ou s'y rattache si elle existe déjà (rejeu des dernières sorties).
    pub fn open(&self, id: &str, spec: &Spec, sink: Sink) -> Result<Opened, String> {
        let mut map = self
            .sessions
            .lock()
            .map_err(|_| "état terminal verrouillé")?;
        if let Some(s) = map.get(id) {
            let _ = s.master.resize(size(spec.cols, spec.rows));
            let (replay, end) = s.ring.lock().map_err(|_| "anneau verrouillé")?.snapshot();
            return Ok(Opened {
                replay,
                end,
                alive: s.alive.load(Ordering::SeqCst),
            });
        }

        let pair = native_pty_system()
            .openpty(size(spec.cols, spec.rows))
            .map_err(|e| format!("Pseudo-terminal indisponible : {e}"))?;
        let mut cmd = CommandBuilder::new(&spec.program);
        cmd.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            cmd.cwd(cwd);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        let mut wslenv = std::env::var("WSLENV").ok();
        for (k, v) in &spec.env {
            cmd.env(k, v);
            wslenv = Some(wslenv_with(wslenv.as_deref(), k));
        }
        if let Some(w) = wslenv.filter(|_| !spec.env.is_empty()) {
            cmd.env("WSLENV", w);
        }
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Impossible de lancer {} : {e}", spec.program))?;
        drop(pair.slave);
        let killer = child.clone_killer();
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Lecture du terminal impossible : {e}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Écriture du terminal impossible : {e}"))?;

        let ring = Arc::new(Mutex::new(Ring::new(BUF_CAP)));
        let alive = Arc::new(AtomicBool::new(true));

        // Lecteur : la sortie alimente l'anneau puis l'affichage.
        {
            let (ring, sink, id) = (ring.clone(), sink.clone(), id.to_string());
            std::thread::spawn(move || {
                let mut chunk = [0u8; 8192];
                while let Ok(n) = reader.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    let end = match ring.lock() {
                        Ok(mut r) => r.push(&chunk[..n]),
                        Err(_) => break,
                    };
                    sink(Event::Data {
                        id: id.clone(),
                        bytes: chunk[..n].to_vec(),
                        end,
                    });
                }
            });
        }
        // Guetteur : sous ConPTY la lecture ne se ferme pas toujours à la fin du processus.
        {
            let (alive, id) = (alive.clone(), id.to_string());
            std::thread::spawn(move || {
                let _ = child.wait();
                alive.store(false, Ordering::SeqCst);
                sink(Event::Exit { id });
            });
        }

        map.insert(
            id.to_string(),
            Session {
                master: pair.master,
                writer,
                killer,
                ring,
                alive,
            },
        );
        Ok(Opened {
            replay: Vec::new(),
            end: 0,
            alive: true,
        })
    }

    pub fn write(&self, id: &str, data: &[u8]) -> Result<(), String> {
        let mut map = self
            .sessions
            .lock()
            .map_err(|_| "état terminal verrouillé")?;
        let s = map.get_mut(id).ok_or("Terminal inconnu.")?;
        s.writer.write_all(data).map_err(|e| e.to_string())?;
        s.writer.flush().map_err(|e| e.to_string())
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let map = self
            .sessions
            .lock()
            .map_err(|_| "état terminal verrouillé")?;
        let s = map.get(id).ok_or("Terminal inconnu.")?;
        s.master.resize(size(cols, rows)).map_err(|e| e.to_string())
    }

    /// Termine le processus et oublie la session (inconnue = sans effet).
    pub fn close(&self, id: &str) {
        if let Ok(mut map) = self.sessions.lock() {
            if let Some(mut s) = map.remove(id) {
                let _ = s.killer.kill();
            }
        }
    }

    pub fn kill_all(&self) {
        if let Ok(mut map) = self.sessions.lock() {
            for (_, mut s) in map.drain() {
                let _ = s.killer.kill();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn anneau_garde_la_fin_et_compte_le_total() {
        let mut r = Ring::new(4);
        assert_eq!(r.push(b"ab"), 2);
        assert_eq!(r.push(b"cdef"), 6);
        let (bytes, total) = r.snapshot();
        assert_eq!(bytes, b"cdef");
        assert_eq!(total, 6);
    }

    fn collect(rx: &mpsc::Receiver<Event>, needle: &str) -> bool {
        let mut seen = Vec::new();
        while let Ok(ev) = rx.recv_timeout(Duration::from_secs(5)) {
            if let Event::Data { bytes, .. } = ev {
                seen.extend(bytes);
                if String::from_utf8_lossy(&seen).contains(needle) {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(unix)]
    #[test]
    fn session_ecrit_lit_puis_se_ferme() {
        let st = PtyState::default();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |e| {
            let _ = tx.lock().unwrap().send(e);
        });
        let spec = Spec {
            program: "cat".into(),
            args: vec![],
            cwd: None,
            cols: 80,
            rows: 24,
            env: vec![],
        };
        let o = st.open("t1", &spec, sink.clone()).unwrap();
        assert!(o.alive && o.replay.is_empty());
        st.write("t1", b"bonjour\n").unwrap();
        assert!(collect(&rx, "bonjour"));
        st.resize("t1", 100, 30).unwrap();
        // rattachement : la session existe, la sortie est rejouée
        let again = st.open("t1", &spec, sink).unwrap();
        assert!(again.alive);
        assert!(String::from_utf8_lossy(&again.replay).contains("bonjour"));
        assert!(again.end >= again.replay.len() as u64);
        st.close("t1");
        assert!(st.write("t1", b"x").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn fin_du_processus_signalee() {
        let st = PtyState::default();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |e| {
            let _ = tx.lock().unwrap().send(e);
        });
        let spec = Spec {
            program: "sh".into(),
            args: vec!["-c".into(), "printf fini".into()],
            cwd: None,
            cols: 80,
            rows: 24,
            env: vec![],
        };
        st.open("t2", &spec, sink).unwrap();
        let mut exited = false;
        while let Ok(ev) = rx.recv_timeout(Duration::from_secs(5)) {
            if matches!(ev, Event::Exit { .. }) {
                exited = true;
                break;
            }
        }
        assert!(exited);
    }

    #[test]
    fn programme_introuvable_donne_une_erreur() {
        let st = PtyState::default();
        let sink: Sink = Arc::new(|_| {});
        let spec = Spec {
            program: "programme-qui-n-existe-pas-xyz".into(),
            args: vec![],
            cwd: None,
            cols: 80,
            rows: 24,
            env: vec![],
        };
        assert!(st.open("t3", &spec, sink).is_err());
    }

    #[test]
    fn wslenv_ajoute_sans_doublon() {
        assert_eq!(wslenv_with(None, "PROMPT_COMMAND"), "PROMPT_COMMAND");
        assert_eq!(wslenv_with(Some(""), "PROMPT_COMMAND"), "PROMPT_COMMAND");
        assert_eq!(
            wslenv_with(Some("A/p"), "PROMPT_COMMAND"),
            "A/p:PROMPT_COMMAND"
        );
        assert_eq!(
            wslenv_with(Some("PROMPT_COMMAND/u:A"), "PROMPT_COMMAND"),
            "PROMPT_COMMAND/u:A"
        );
    }

    #[cfg(unix)]
    #[test]
    fn variables_transmises_au_processus() {
        let st = PtyState::default();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |e| {
            let _ = tx.lock().unwrap().send(e);
        });
        let spec = Spec {
            program: "sh".into(),
            args: vec![
                "-c".into(),
                "printf '[%s]' \"$PROMPT_COMMAND\"; sleep 1".into(),
            ],
            cwd: None,
            cols: 80,
            rows: 24,
            env: vec![("PROMPT_COMMAND".into(), "echo hi".into())],
        };
        st.open("e1", &spec, sink).unwrap();
        assert!(collect(&rx, "[echo hi]"));
        st.close("e1");
    }
}
