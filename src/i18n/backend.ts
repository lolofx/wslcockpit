// Messages renvoyés par le backend Rust (en français : format d'échange). On les reconnaît par motif et on les affiche
// dans la langue courante ; un message inconnu (sortie brute de wsl.exe, erreur système) est affiché tel quel.
import type { Params } from './core.ts'

type Rule = { key: string; re: RegExp }

const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
// Écrit le motif depuis le texte français : {nom} devient un groupe nommé.
const rule = (key: string, fr: string): Rule => ({
  key,
  re: new RegExp('^' + esc(fr).replace(/\\\{(\w+)\\\}/g, (_m, n: string) => `(?<${n}>.+?)`) + '$', 's'),
})

export const RULES: Rule[] = [
  rule('backend.busy', 'Une opération est déjà en cours.'),
  rule('backend.cancelled', 'Opération annulée.'),
  rule('backend.stopFirst', 'Arrête la distribution avant cette opération.'),
  rule('backend.gone', "Cette distribution n'existe plus."),
  rule('backend.wslFailed', 'wsl.exe a échoué (code {code}) : {message}'),
  rule('backend.nameMismatch', 'Le nom saisi ne correspond pas : suppression refusée.'),
  rule('backend.exported', 'Exporté vers {path}'),
  rule('backend.imported', '« {name} » importée.'),
  rule('backend.cloned', '« {source} » clonée en « {name} ».'),
  rule('backend.alreadyVersion', '« {name} » est déjà en WSL{version}.'),
  rule('backend.converted', '« {name} » est maintenant en WSL{version}.'),
  rule('backend.sparseOn', 'Mode sparse activé pour « {name} ».'),
  rule('backend.sparseOff', 'Mode sparse désactivé pour « {name} ».'),
  rule('backend.installed', "« {name} » installée. Ouvre un terminal pour créer l'utilisateur."),
  rule('backend.wslUpToDateVersion', 'WSL est à jour (version {version}).'),
  rule('backend.wslUpToDate', 'WSL est à jour.'),
  rule('backend.stage.export', 'Export en cours…'),
  rule('backend.stage.import', 'Import en cours…'),
  rule('backend.stage.clone1', '1/2 Export de la source…'),
  rule('backend.stage.clone2', '2/2 Import de la copie…'),
  rule('backend.stage.convert', 'Conversion en WSL{version}…'),
  rule('backend.stage.sparse', 'Changement du mode sparse…'),
  rule('backend.stage.install', 'Téléchargement et installation de {distro}…'),
  rule('backend.stage.update', 'Mise à jour de WSL…'),
  rule('backend.archiveMissing', 'Archive introuvable.'),
  rule('backend.badVersion', 'Version WSL invalide (1 ou 2).'),
  rule('backend.parentMissing', "Le dossier parent du dossier choisi doit exister (l'archive temporaire y est écrite)."),
  rule('backend.sparseUnavailable', 'Mode sparse indisponible : WSL {version} installé, 2.0.0 ou plus requis (wsl --update).'),
  rule('backend.sparseUnknown', 'Mode sparse indisponible : version de WSL inconnue (wsl --update).'),
  rule('backend.customTooOld', 'Nom et dossier personnalisés demandent WSL 2.4.4 ou plus récent (wsl --update).'),
  rule('backend.customUnknown', 'Version de WSL inconnue : nom et dossier personnalisés indisponibles (wsl --update).'),
  rule('backend.nameLength', 'Nom invalide : 1 à 64 caractères, sans tiret au début.'),
  rule('backend.nameChars', "Nom invalide : lettres, chiffres, _ - . seulement (pas d'espace)."),
  rule('backend.nameExists', 'Une distribution « {name} » existe déjà.'),
  rule('backend.pathIsFile', 'Le chemin choisi est un fichier, pas un dossier.'),
  rule('backend.dirNotEmpty', "Le dossier existe déjà et n'est pas vide : choisis-en un autre."),
  rule('backend.dirUnreadable', 'Dossier illisible : {error}'),
  rule('backend.tarExt', "Le fichier d'export doit se terminer par .tar."),
  rule('backend.fileExists', 'Ce fichier existe déjà : il ne sera pas écrasé.'),
  rule('backend.destMissing', "Le dossier de destination n'existe pas."),
  rule('backend.memory', 'Mémoire invalide : indique par exemple 8GB ou 4096MB.'),
  rule('backend.swap', 'Swap invalide : indique par exemple 4GB, ou 0 pour le désactiver.'),
  rule('backend.processors', 'Processeurs invalide : un nombre entier supérieur ou égal à 1.'),
  rule('backend.networking', 'Mode réseau invalide : nat, mirrored, bridged, virtioproxy ou none.'),
  rule('backend.idle', "Délai d'inactivité invalide : un nombre de millisecondes (-1 pour désactiver)."),
  rule('backend.bool', 'systemd : true ou false.'),
  rule('backend.writeSettings', 'Écriture des réglages impossible : {error}'),
  rule('backend.autostart', 'Impossible de modifier le démarrage avec Windows.'),
  rule('backend.pathRefused', 'Chemin refusé.'),
  rule('backend.linkRefused', 'Lien refusé.'),
  rule('backend.terminalUnknown', 'Terminal inconnu.'),
  rule('backend.ptyUnavailable', 'Pseudo-terminal indisponible : {error}'),
  rule('backend.ptyLaunch', 'Impossible de lancer {program} : {error}'),
  rule('backend.ptyRead', 'Lecture du terminal impossible : {error}'),
  rule('backend.ptyWrite', 'Écriture du terminal impossible : {error}'),
  rule('backend.gitFailed', 'git a échoué ({code}) : {error}'),
]

/** Traduit un message du backend ; sans motif connu, le texte reste tel quel. */
export function backendMsg(text: string, tr: (key: string, params?: Params) => string): string {
  const s = String(text)
  for (const { key, re } of RULES) {
    const m = re.exec(s)
    if (m) return tr(key, { ...(m.groups ?? {}) })
  }
  return s
}
