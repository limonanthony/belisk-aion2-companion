// Updates and changelog round (2026-10-09): drawing shared by toolbar.html,
// whatsnew.html and settings.html. Builds on the approved design, loaded
// unchanged: the toolbar is ../approved/hub.js's own drawing, with this
// round's update slot added; the main window is ../approved/app.js's shell.
//
// Everything is invented: version numbers (the reset number, 0.1.0 or 1.0.0,
// is not decided), dates, sizes, times and the changelog's text. Nothing is
// downloaded, saved or sent anywhere.
//
// Icons: Lucide (https://lucide.dev), ISC License; paths copied from
// gpui-kit-assets 0.6.6 (download, rotate-cw, scroll-text, cloud-off, pause,
// chevron-left, loader-circle); the rest come from kit.js / app.js.
(function () {
  "use strict";

  // The product name, in one place: the Fluent term `-app-name`
  // (crates/app/i18n/*/aion_2_dps_meter.ftl). A rename changes only this.
  const APP = "AION 2 DPS Meter";

  const SVG = {
    download: '<path d="M12 15V3"/><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/>',
    rotateCw: '<path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/>',
    scrollText:
      '<path d="M15 12h-5"/><path d="M15 8h-5"/><path d="M19 17V5a2 2 0 0 0-2-2H4"/><path d="M8 21h12a2 2 0 0 0 2-2v-1a1 1 0 0 0-1-1H11a1 1 0 0 0-1 1v1a2 2 0 1 1-4 0V5a2 2 0 1 0-4 0v2a1 1 0 0 0 1 1h3"/>',
    cloudOff: '<path d="M10.94 5.274A7 7 0 0 1 15.71 10h1.79a4.5 4.5 0 0 1 4.222 6.057"/><path d="M18.796 18.81A4.5 4.5 0 0 1 17.5 19H9A7 7 0 0 1 5.79 5.78"/><path d="m2 2 20 20"/>',
    pause: '<rect x="14" y="3" width="5" height="18" rx="1"/><rect x="5" y="3" width="5" height="18" rx="1"/>',
    chevronLeft: '<path d="m15 18-6-6 6-6"/>',
    chevronRight: '<path d="m9 18 6-6-6-6"/>',
    chevronDown: '<path d="m6 9 6 6 6-6"/>',
    loader: '<path d="M21 12a9 9 0 1 1-6.219-8.56"/>',
    clock: '<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>',
    alert: '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
  };

  function icon(name, cls) {
    if (SVG[name]) {
      return `<svg class="i${cls ? " " + cls : ""}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${SVG[name]}</svg>`;
    }
    const lib = window.Kit || window.App;
    return lib.icon(name, cls);
  }

  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  // ---- Versions (invented) --------------------------------------------------
  // The build runs 1.5.1; GitHub's latest release is 1.6.0, 4.2 MB.
  const RUNNING = "1.5.1";
  const NEXT = "1.6.0";
  const SIZE_MB = 4.2;
  const CAP_MB = 1; // the speed cap, MB/s (Proposed)

  // ---- The changelog files (invented text) -----------------------------------
  // CHANGELOG.md and CHANGELOG.fr.md as the app would embed them. The subset:
  // "## [x.y.z] - YYYY-MM-DD", one summary paragraph, "- " bullets, **bold**.
  const MD = {
    en: `# Changelog

## [1.6.0] - 2026-10-21

Updates arrive inside the app, and a failed boss attempt gets its own record.

- **Updates**: when a new version is out, an icon appears in the toolbar. One click downloads it, a second restarts into it. Nothing installs without your click.
- A wipe ends the attempt: when the boss is back to full health, the failed try is saved to History, marked as a wipe.
- **What's new** opens once after an update and lists every version since the one you last ran.
- The same buff from the same player shows as one row instead of several.

## [1.5.1] - 2026-10-14

Two fixes.

- The Shugo Festival reminder for 00:00 shows again.
- Settings opens in front of the game instead of behind it.

## [1.5.0] - 2026-10-10

A toolbar becomes the home of every tool.

- A small toolbar opens at launch. The DPS meter, PvP meter, to-dos, fight history, reminders and settings open from it.
- **Resources window**: Odyle Energy, Shugo keys, Invasion keys, Nightmare and the Daily Dungeon in one place, filled from the game.
- The toolbar warns when a resource is full shortly before it refills.

## [1.4.0] - 2026-10-07

A report at the end of every boss fight.

- After a boss dies, open its report from the summary card or from History: damage, DPS, healing, damage taken and deaths for every player.
- Click a name for that player's skills and buffs.

## [1.3.1] - 2026-10-04

Fixes after 1.3.0.

- Closing the meter with × ends the app again, instead of leaving it running.
- Skills that showed as a number have their names.

## [1.3.0] - 2026-10-02

Event reminders.

- A small card appears a minute before the Shugo Festival, Dimensional Invasions and rift portals. Pick the events in **Settings › Reminders**.

## [1.2.0] - 2026-09-28

The app speaks French.

- Every screen is in French. Change the language in **Settings › General**.
- Skills and monsters show their French names where they are known.

## [1.1.0] - 2026-09-24

Daily and weekly to-dos.

- A to-do window lists what resets each day and each week, for each character.

## [1.0.1] - 2026-09-21

One fix.

- The meter starts at your first hit on a monster instead of waiting.

## [1.0.0] - 2026-09-19

The first release of this app.

- A new overlay with sharper text, and the Session and Target boards.
- Personal bests for each boss and character, and your pace against them during the fight.
`,
    fr: `# Journal des modifications

## [1.6.0] - 2026-10-21

Les mises à jour arrivent dans l'application, et une tentative ratée sur un boss a son propre enregistrement.

- **Mises à jour** : quand une nouvelle version sort, une icône apparaît dans la barre d'outils. Un clic la télécharge, un second redémarre dessus. Rien ne s'installe sans votre clic.
- Un wipe termine la tentative : quand le boss revient à pleine vie, l'essai raté est enregistré dans l'historique, marqué comme wipe.
- **Nouveautés** s'ouvre une fois après une mise à jour et liste toutes les versions depuis la dernière que vous avez lancée.
- Le même buff du même joueur s'affiche sur une seule ligne au lieu de plusieurs.

## [1.5.1] - 2026-10-14

Deux corrections.

- Le rappel du Shugo Festival de 00:00 s'affiche de nouveau.
- Les paramètres s'ouvrent devant le jeu et non plus derrière.

## [1.5.0] - 2026-10-10

Une barre d'outils devient la maison de tous les outils.

- Une petite barre d'outils s'ouvre au lancement. Le compteur DPS, le compteur JcJ, les tâches, l'historique des combats, les rappels et les paramètres s'ouvrent depuis elle.
- **Fenêtre Ressources** : Odyle Energy, Shugo keys, Invasion keys, Nightmare et le Daily Dungeon au même endroit, remplis depuis le jeu.
- La barre d'outils prévient quand une ressource est pleine peu avant sa recharge.

## [1.4.0] - 2026-10-07

Un rapport à la fin de chaque combat de boss.

- Après la mort d'un boss, ouvrez son rapport depuis la carte de résumé ou depuis l'historique : dégâts, DPS, soins, dégâts subis et morts de chaque joueur.
- Cliquez sur un nom pour les compétences et les buffs de ce joueur.

## [1.3.1] - 2026-10-04

Corrections après la 1.3.0.

- Fermer le compteur avec × quitte de nouveau l'application, au lieu de la laisser tourner.
- Les compétences affichées comme un nombre ont leur nom.

## [1.3.0] - 2026-10-02

Rappels d'événements.

- Une petite carte apparaît une minute avant le Shugo Festival, les Invasions dimensionnelles et les portails de faille. Choisissez les événements dans **Paramètres › Rappels**.

## [1.2.0] - 2026-09-28

L'application parle français.

- Tous les écrans sont en français. Changez la langue dans **Paramètres › Général**.
- Les compétences et les monstres affichent leur nom français quand il est connu.

## [1.1.0] - 2026-09-24

Tâches quotidiennes et hebdomadaires.

- Une fenêtre de tâches liste ce qui se réinitialise chaque jour et chaque semaine, pour chaque personnage.

## [1.0.1] - 2026-09-21

Une correction.

- Le compteur démarre à votre premier coup sur un monstre au lieu d'attendre.

## [1.0.0] - 2026-09-19

La première version de cette application.

- Un nouvel overlay au texte plus net, avec les tableaux Session et Cible.
- Records perso pour chaque boss et chaque personnage, et votre rythme face à eux pendant le combat.
`,
  };

  // ---- The app's parser for the subset (what the Rust parser would do) ------
  // Anything outside the subset throws with its line number: in the build, the
  // same check runs in cargo test on both files, so a mistake fails the test
  // instead of rendering badly.
  const HEAD = /^## \[(\d+\.\d+\.\d+)\] - (\d{4}-\d{2}-\d{2})$/;

  function inline(text, line) {
    const parts = text.split("**");
    if (parts.length % 2 === 0) throw new Error(`line ${line}: unclosed **`);
    if (/[`_<>]|\[|\]\(|(^|[^*])\*([^*]|$)/.test(parts.join(""))) throw new Error(`line ${line}: only **bold** is supported`);
    return parts.map((p, i) => (i % 2 ? `<b>${esc(p)}</b>` : esc(p))).join("");
  }

  function parse(md) {
    const out = [];
    let cur = null;
    const lines = md.replace(/\r\n/g, "\n").split("\n");
    lines.forEach((raw, i) => {
      const n = i + 1;
      const line = raw.trimEnd();
      if (!line) return;
      if (n === 1 && /^# \S/.test(line)) return;
      const h = HEAD.exec(line);
      if (h) {
        cur = { version: h[1], date: h[2], summary: "", bullets: [] };
        out.push(cur);
        return;
      }
      if (!cur) throw new Error(`line ${n}: text before the first version`);
      if (line.startsWith("- ")) {
        cur.bullets.push(inline(line.slice(2), n));
        return;
      }
      if (line.startsWith("  ") && cur.bullets.length) {
        cur.bullets[cur.bullets.length - 1] += " " + inline(line.trim(), n);
        return;
      }
      if (/^(#|>|\* |\+ |\d+\. |\|)/.test(line)) throw new Error(`line ${n}: not in the subset`);
      if (cur.bullets.length) throw new Error(`line ${n}: the summary comes before the bullets`);
      cur.summary += (cur.summary ? " " : "") + inline(line, n);
    });
    return out;
  }

  // Parity, as the build's test: same versions and bullet counts per version.
  function parity(a, b) {
    if (a.length !== b.length) return `different number of versions (${a.length} and ${b.length})`;
    for (let i = 0; i < a.length; i++) {
      if (a[i].version !== b[i].version) return `version ${a[i].version} against ${b[i].version}`;
      if (a[i].bullets.length !== b[i].bullets.length) return `${a[i].version}: ${a[i].bullets.length} bullets against ${b[i].bullets.length}`;
    }
    return "";
  }

  const LOG = { en: parse(MD.en), fr: parse(MD.fr) };

  const cmp = (a, b) => {
    const x = a.split(".").map(Number);
    const y = b.split(".").map(Number);
    for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] - y[i];
    return 0;
  };

  // The versions the build of `running` knows (its embedded file), newest first.
  const known = (lang, running) => LOG[lang].filter((v) => cmp(v.version, running) <= 0);
  // What's new: newer than the last version this player ran, up to the running one.
  const newer = (lang, last, running) => LOG[lang].filter((v) => cmp(v.version, last) > 0 && cmp(v.version, running) <= 0 && v.bullets.length);

  const MONTHS = {
    en: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
    fr: ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."],
  };
  const date = (iso, lang, year = true) => {
    const [y, m, d] = iso.split("-").map(Number);
    return `${d} ${MONTHS[lang][m - 1]}${year ? " " + y : ""}`;
  };

  const mb = (n, lang) => (lang === "fr" ? `${String(n.toFixed(1)).replace(".", ",")} Mo` : `${n.toFixed(1)} MB`);
  const pct = (n, lang) => (lang === "fr" ? `${n}\u202f%` : `${n}%`);

  // ---- Strings (English; French drafted in the approved French style, real
  // text through Fluent) ---------------------------------------------------------
  const T = {
    en: {
      // Toolbar tooltips
      tAvail: (v) => [`Update available: ${v}`, "Click to download it. It installs only when you restart."],
      tFailed: (v) => [`Update available: ${v}`, "The last download failed its check and was deleted. Click to try again."],
      tDown: (v, p) => [`Downloading ${v} · ${p}`, "Speed-capped, paused while you fight. Click for details."],
      tPaused: (v, p) => [`Download paused · ${p}`, "Resumes when the fight ends. Click for details."],
      tCheck: (v) => [`Checking ${v}…`, "Making sure the download is complete and genuine."],
      tReady: (v) => [`Restart to update to ${v}`, "Closes and opens again in the new version. Nothing is lost."],
      tReadyFight: (v) => [`Restart to update to ${v}`, "You are fighting: restarting now splits this fight in History."],
      // What's new window
      wnTitle: "What's new",
      wnChip: (v) => `Updated to ${v}`,
      wnSince: (v) => `Since <b>${v}</b>, the last version you ran`,
      wnFull: "Full changelog",
      wnClose: "Close",
      // Settings › About › Updates
      grp: "Updates",
      grpNote: "Asks GitHub for the latest release; nothing about you or your game is sent.",
      upToDate: "Up to date",
      upToDateDesc: (v) => `${v} is the latest version. Checked at 21:02; checked again at launch and every hour.`,
      justChecked: (v) => `${v} is the latest version. Checked just now; you can check again in a minute.`,
      checking: "Checking for updates…",
      checkingDesc: "Asking GitHub for the latest release.",
      checkNow: "Check now",
      checkingBadge: "Checking",
      avail: (v) => `Version ${v} is available`,
      availDesc: (d, s) => `Released ${d} · ${s}. Its notes are on GitHub until you update; then What's new shows them.`,
      notes: "Release notes",
      update: (v) => `Update to ${v}`,
      down: (v) => `Downloading ${v}`,
      downDesc: (a, b) => `${a} of ${b} · at most ${CAP_MB} MB/s, paused while you fight`,
      paused: "Download paused",
      pausedDesc: (a, b) => `${a} of ${b} · resumes when the fight ends`,
      cancel: "Cancel",
      verify: "Checking the download",
      verifyDesc: (v) => `Making sure ${v} is complete and genuine.`,
      ready: (v) => `${v} is ready to install`,
      readyDesc: (v) => `Downloaded and checked. Restart to install it: the app closes and opens again in ${v}, with your settings, to-dos and history.`,
      restart: "Restart to update",
      failedMsg: "The last download did not pass its check (signature mismatch), so it was deleted. Nothing was installed.",
      tryAgain: "Try again",
      offline: "Could not check for updates",
      offlineDesc: "Last successful check at 18:02.",
      offlineMsg: "GitHub could not be reached: no internet, or GitHub is down. Next try at 22:02.",
      limited: "Could not check for updates",
      limitedMsg: "GitHub allows 60 update checks an hour per internet address, and your network has used them up (other PCs on the same connection count too). Next check at 22:14.",
      auto: "Download updates automatically",
      autoDesc: "New versions download in the background: speed-capped, and paused while you fight. Installing always waits for your Restart.",
      log: "Changelog",
      logDesc: "What changed in each version.",
      logOpen: "Open changelog",
      // Changelog page
      back: "About",
      logLead: (v) => `What changed in each version. You have ${v}.`,
      older: "Show older",
      shown: (a, b) => `${a} of ${b} versions`,
      all: (b) => `All ${b} versions`,
      // About (real strings, crates/app/i18n/en)
      aboutDesc: "A real-time DPS meter for AION 2, built for the global servers.",
      source: "Source and notice",
      licence: "Licence: GPL-3.0-only",
      licenceText: "Free software: you may share and change it under the GNU General Public License, version 3.",
      readLicence: "Read the licence",
      credits: "Credits",
      forked: "Forked from",
      upstream: "Upstream",
      aether: "This meter is a fork of Aether, by Helveticxa. GPL-3.0-only.",
      noia2: "By zdyoung. Aether, which this meter is forked from, is itself a fork of NOIA2: the capture pipeline, packet parsers, overlay and game-data catalogues come from it. GPL-3.0-only.",
      storage: "Storage",
      stored: "Stored on this PC",
      storedLine: "Settings 6 KB · Fight history 14 records, 2.1 MB · Recordings 5 files, 11.8 MB",
      openFolder: "Open folder",
      reset: "Reset settings",
      resetText: "Settings go back to their defaults.",
      resetBtn: "Reset settings…",
    },
    fr: {
      tAvail: (v) => [`Mise à jour disponible : ${v}`, "Cliquer pour la télécharger. Elle s'installe seulement au redémarrage."],
      tFailed: (v) => [`Mise à jour disponible : ${v}`, "Le dernier téléchargement n'a pas passé la vérification. Cliquer pour réessayer."],
      tDown: (v, p) => [`Téléchargement de ${v} · ${p}`, "Débit limité, en pause pendant les combats. Cliquer pour les détails."],
      tPaused: (v, p) => [`Téléchargement en pause · ${p}`, "Reprend à la fin du combat. Cliquer pour les détails."],
      tCheck: (v) => [`Vérification de ${v}…`, "On vérifie que le fichier est complet et authentique."],
      tReady: (v) => [`Redémarrer pour passer à ${v}`, "Ferme et rouvre dans la nouvelle version. Rien n'est perdu."],
      tReadyFight: (v) => [`Redémarrer pour passer à ${v}`, "Combat en cours : redémarrer maintenant le coupe en deux dans l'historique."],
      wnTitle: "Nouveautés",
      wnChip: (v) => `Mis à jour en ${v}`,
      wnSince: (v) => `Depuis la <b>${v}</b>, la dernière version que vous avez lancée`,
      wnFull: "Journal complet",
      wnClose: "Fermer",
      grp: "Mises à jour",
      grpNote: "Demande la dernière version à GitHub ; rien sur vous ni sur votre jeu n'est envoyé.",
      upToDate: "À jour",
      upToDateDesc: (v) => `${v} est la dernière version. Vérifié à 21:02 ; vérifié à nouveau au lancement et toutes les heures.`,
      justChecked: (v) => `${v} est la dernière version. Vérifié à l'instant ; nouvelle vérification possible dans une minute.`,
      checking: "Recherche de mises à jour…",
      checkingDesc: "Demande la dernière version à GitHub.",
      checkNow: "Vérifier maintenant",
      checkingBadge: "Recherche",
      avail: (v) => `La version ${v} est disponible`,
      availDesc: (d, s) => `Publiée le ${d} · ${s}. Ses notes sont sur GitHub jusqu'à la mise à jour ; ensuite, Nouveautés les affiche.`,
      notes: "Notes de version",
      update: (v) => `Mettre à jour en ${v}`,
      down: (v) => `Téléchargement de ${v}`,
      downDesc: (a, b) => `${a} sur ${b} · ${CAP_MB} Mo/s au plus, en pause pendant les combats`,
      paused: "Téléchargement en pause",
      pausedDesc: (a, b) => `${a} sur ${b} · reprend à la fin du combat`,
      cancel: "Annuler",
      verify: "Vérification du téléchargement",
      verifyDesc: (v) => `On vérifie que ${v} est complète et authentique.`,
      ready: (v) => `${v} est prête à être installée`,
      readyDesc: (v) => `Téléchargée et vérifiée. Redémarrez pour l'installer : l'application se ferme et se rouvre en ${v}, avec vos paramètres, tâches et historique.`,
      restart: "Redémarrer pour mettre à jour",
      failedMsg: "Le dernier téléchargement n'a pas passé la vérification (signature incorrecte) ; il a été supprimé. Rien n'a été installé.",
      tryAgain: "Réessayer",
      offline: "Impossible de vérifier les mises à jour",
      offlineDesc: "Dernière vérification réussie à 18:02.",
      offlineMsg: "GitHub est injoignable : pas d'internet, ou GitHub est en panne. Prochain essai à 22:02.",
      limited: "Impossible de vérifier les mises à jour",
      limitedMsg: "GitHub autorise 60 vérifications par heure et par adresse internet, et votre réseau les a épuisées (les autres PC sur la même connexion comptent aussi). Prochaine vérification à 22:14.",
      auto: "Télécharger les mises à jour automatiquement",
      autoDesc: "Les nouvelles versions se téléchargent en arrière-plan : débit limité, et en pause pendant les combats. L'installation attend toujours votre clic sur Redémarrer.",
      log: "Journal des modifications",
      logDesc: "Ce qui a changé à chaque version.",
      logOpen: "Ouvrir le journal",
      back: "À propos",
      logLead: (v) => `Ce qui a changé à chaque version. Vous avez la ${v}.`,
      older: "Afficher les plus anciennes",
      shown: (a, b) => `${a} versions sur ${b}`,
      all: (b) => `Les ${b} versions`,
      aboutDesc: "Un compteur de DPS en temps réel pour AION 2, conçu pour les serveurs internationaux.",
      source: "Source et mentions",
      licence: "Licence : GPL-3.0-only",
      licenceText: "Logiciel libre : vous pouvez le partager et le modifier selon la GNU General Public License, version 3.",
      readLicence: "Lire la licence",
      credits: "Crédits",
      forked: "Dérivé de",
      upstream: "Origine",
      aether: "Ce compteur est un fork d'Aether, par Helveticxa. GPL-3.0-only.",
      noia2: "Par zdyoung. Aether, dont ce compteur est un fork, est lui-même un fork de NOIA2 : la chaîne de capture, les analyseurs de paquets, l'overlay et les catalogues de données du jeu en viennent. GPL-3.0-only.",
      storage: "Stockage",
      stored: "Stocké sur ce PC",
      storedLine: "Paramètres 6 Ko · Historique des combats 14 enregistrements, 2,1 Mo · Enregistrements 5 fichiers, 11,8 Mo",
      openFolder: "Ouvrir le dossier",
      reset: "Réinitialiser les paramètres",
      resetText: "Les paramètres reviennent à leurs valeurs par défaut.",
      resetBtn: "Réinitialiser les paramètres…",
    },
  };

  // ---- Toolbar: the update slot ------------------------------------------------
  // o.up: { state: "available" | "failed" | "downloading" | "paused" |
  //   "verifying" | "ready", pct, fight }; o.upHover, o.upTip.
  function updateBtn(o) {
    const u = o.up;
    if (!u) return "";
    const L = T[o.lang || "en"];
    const p = pct(u.pct || 0, o.lang);
    const busy = u.state === "downloading" || u.state === "paused" || u.state === "verifying";
    let ic = "download";
    let tip;
    const cls = ["hb-btn", "hb-up"];
    if (u.state === "available") tip = L.tAvail(NEXT);
    else if (u.state === "failed") tip = L.tFailed(NEXT);
    else if (u.state === "downloading") tip = L.tDown(NEXT, p);
    else if (u.state === "paused") tip = L.tPaused(NEXT, p);
    else if (u.state === "verifying") tip = L.tCheck(NEXT);
    else if (u.state === "ready") {
      ic = "rotateCw";
      cls.push("is-ready");
      tip = u.fight ? L.tReadyFight(NEXT) : L.tReady(NEXT);
    }
    if (busy) cls.push("is-busy");
    if (u.state === "paused") cls.push("is-paused");
    if (o.upHover) cls.push("is-hover");
    if (o.upFocus) cls.push("is-focus");
    const fill = u.state === "verifying" ? 100 : u.pct || 0;
    const prog = busy ? `<span class="hb-prog" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${fill}"><i style="width:${fill}%"></i></span>` : "";
    const [a, b] = tip;
    const tipHtml = `<span class="hb-tip${o.upTip ? " is-on" : ""}" role="tooltip">${a}<span>${b}</span></span>`;
    return `<button type="button" class="${cls.join(" ")}" data-up="${u.state}" aria-label="${esc(a)}">${icon(ic)}${prog}${tipHtml}</button>`;
  }

  // The approved toolbar (../approved/hub.js), its name taken from APP, with
  // the update slot first in the end group (before fold), and after the
  // warning when folded.
  function toolbar(ctx, o = {}) {
    let html = Hub.toolbar(ctx, o).replace(/Kisk/g, APP);
    const slot = updateBtn(o);
    if (!slot) return html;
    if (o.folded) return html.replace(/<\/div>$/, `${slot}</div>`);
    const fold = '<button type="button" class="hb-btn is-end" data-hb="fold"';
    return html.replace(fold, slot + fold);
  }

  // ---- What's new window ---------------------------------------------------------
  // o: { lang, last (the last version this player ran), running, scroll (px),
  //      hover: "close" | "full" }
  function whatsNew(ctx, o = {}) {
    const lang = o.lang || "en";
    const L = T[lang];
    const running = o.running || NEXT;
    const list = newer(lang, o.last || RUNNING, running);
    const versions = list
      .map(
        (v) =>
          `<section class="wn-ver"><div class="wn-vh"><span class="wn-v">${v.version}</span><span class="wn-d">${date(v.date, lang)}</span></div><div class="wn-sum">${v.summary}</div><ul class="wn-list">${v.bullets
            .map((b) => `<li>${b}</li>`)
            .join("")}</ul></section>`
      )
      .join("");
    const close = `<button type="button" class="o-btn${o.hover === "close" ? " is-hot" : ""}" aria-label="${L.wnClose}">${icon("close")}</button>`;
    const head = `<div class="o-head"><span class="o-title">${icon("scrollText")}${L.wnTitle}</span><span class="o-chip is-good">${icon("check")}${L.wnChip(running)}</span><span class="grow"></span>${close}</div>`;
    const since = list.length > 1 ? `<div class="wn-since">${L.wnSince(o.last)}</div>` : "";
    const long = list.length > 1;
    // o.scroll: drawn scrolled to the end of the list.
    const thumb = long ? `<span class="o-thumb" style="--t:${o.scroll ? 152 : 2}px;--h:164px"></span>` : "";
    const body = `<div class="o-scroll wn-body${long ? " has-thumb" : ""}${o.scroll ? " is-end" : ""}"><div class="wn-inner">${versions}</div>${thumb}</div>`;
    const foot = `<div class="o-foot"><span class="grow"></span><button type="button" class="o-tbtn is-quiet${o.hover === "full" ? " is-hover" : ""}">${icon("settings")}${L.wnFull}${icon("chevronRight")}</button></div>`;
    return `<div class="ov o wn" role="dialog" aria-label="${L.wnTitle}">${head}${since}${body}${foot}</div>`;
  }

  // ---- Settings › About (main window, ../approved/app.js) ----------------------
  // The main window's button (app.js C.btn's markup) with this round's
  // icons too, and an optional trailing icon.
  const btn = (label, { kind = "", ic = "", size = "", trail = "" } = {}) =>
    `<span class="btn${kind ? " is-" + kind : ""}${size ? " is-" + size : ""}" role="button">${ic ? icon(ic) : ""}${label}${trail ? icon(trail) : ""}</span>`;

  // st: uptodate | checking | justchecked | available | downloading | paused |
  //     verifying | ready | failed | offline | limited
  function updatesGroup(ctx, st) {
    const { C } = ctx;
    const lang = ctx.lang;
    const L = T[lang];
    const bar = (p, paused) => `<div class="up-bar${paused ? " is-paused" : ""}" role="progressbar" aria-valuenow="${p}" aria-valuemin="0" aria-valuemax="100"><i style="width:${p}%"></i></div>`;
    const msg = (tone, ic, text) => `<div class="field-msg is-${tone}">${icon(ic)}<span>${text}</span></div>`;
    const check = (disabled) => btn(L.checkNow, { ic: "refresh", kind: disabled ? "disabled" : "" });
    const notes = btn(L.notes, { kind: "ghost", ic: "external" });
    const done = 2.6;
    let label;
    let desc;
    let ctl;
    switch (st) {
      case "checking":
        label = L.checking;
        desc = L.checkingDesc;
        ctl = C.badge(L.checkingBadge, "", "loader") + check(true);
        break;
      case "justchecked":
        label = L.upToDate;
        desc = L.justChecked(RUNNING);
        ctl = check(true);
        break;
      case "available":
        label = L.avail(NEXT);
        desc = L.availDesc(date("2026-10-21", lang, false), mb(SIZE_MB, lang));
        ctl = notes + btn(L.update(NEXT), { kind: "primary", ic: "download" });
        break;
      case "failed":
        label = L.avail(NEXT);
        desc = L.availDesc(date("2026-10-21", lang, false), mb(SIZE_MB, lang)) + msg("bad", "alert", L.failedMsg);
        ctl = notes + btn(L.tryAgain, { kind: "primary", ic: "download" });
        break;
      case "downloading":
        label = L.down(NEXT);
        desc = bar(62) + L.downDesc(mb(done, lang), mb(SIZE_MB, lang));
        ctl = btn(L.cancel, { kind: "ghost" });
        break;
      case "paused":
        label = L.paused;
        desc = bar(62, true) + L.pausedDesc(mb(done, lang), mb(SIZE_MB, lang));
        ctl = btn(L.cancel, { kind: "ghost" });
        break;
      case "verifying":
        label = L.verify;
        desc = bar(100) + L.verifyDesc(NEXT);
        ctl = C.badge(L.checkingBadge, "", "loader");
        break;
      case "ready":
        label = L.ready(NEXT);
        desc = L.readyDesc(NEXT);
        ctl = btn(L.restart, { kind: "primary", ic: "rotateCw" });
        break;
      case "offline":
        label = L.offline;
        desc = L.offlineDesc + msg("warn", "cloudOff", L.offlineMsg);
        ctl = check(false);
        break;
      case "limited":
        label = L.limited;
        desc = L.offlineDesc + msg("warn", "clock", L.limitedMsg);
        ctl = check(true);
        break;
      default:
        label = L.upToDate;
        desc = L.upToDateDesc(RUNNING);
        ctl = check(false);
    }
    const autoOn = st !== "available";
    const rows =
      C.row(label, desc, ctl) +
      C.row(L.auto, L.autoDesc, C.sw(autoOn)) +
      C.row(L.log, L.logDesc, btn(L.logOpen, { trail: "chevronRight" }));
    return `<div class="grp-title"><span style="flex-shrink:0">${L.grp}</span><span class="faint" style="font-weight:400">${L.grpNote}</span></div><div class="grp">${rows}</div>`;
  }

  function about(ctx, st) {
    const { C } = ctx;
    const L = T[ctx.lang];
    const li = (title, desc, ctl = "", tag = "") => `<div class="li"><div class="li-main"><b>${title}${tag}</b><small>${desc}</small></div>${ctl}</div>`;
    const tag = (t) => `<span class="badge is-plain credit-tag">${t}</span>`;
    const head = `<div class="h1">${ctx.t.about}</div>`;
    const first =
      `<div class="grp-title is-first">${APP}</div><div class="grp">` +
      C.row(`${APP} <span class="ver">${RUNNING}</span>`, L.aboutDesc, btn(L.source, { ic: "external" })) +
      C.row(L.licence, L.licenceText, btn(L.readLicence, { ic: "scale" })) +
      `</div>`;
    const credits = `<div class="grp-title">${L.credits}</div><div class="grp">${li("Helveticxa/Aether-Aion2-DPS-meter-Global", L.aether, "", tag(L.forked))}${li("ZDYoung0519/NOIA2", L.noia2, "", tag(L.upstream))}</div>`;
    const storage = `<div class="grp-title">${L.storage}</div><div class="grp">${C.row(L.stored, L.storedLine, btn(L.openFolder, { ic: "folder" }))}${C.row(L.reset, L.resetText, btn(L.resetBtn, { kind: "danger" }))}</div>`;
    return head + first + updatesGroup(ctx, st) + credits + storage;
  }

  // ---- Settings › About › Changelog ----------------------------------------------
  // o: { open: version or "", count: versions shown, hover: version, all }
  function changelog(ctx, o = {}) {
    const { C } = ctx;
    const lang = ctx.lang;
    const L = T[lang];
    const all = known(lang, RUNNING);
    const shown = all.slice(0, o.count || 5);
    const open = o.open === undefined ? all[0].version : o.open;
    const items = shown
      .map((v) => {
        const isOpen = v.version === open;
        const row = `<div class="cl-row${o.hover === v.version ? " is-hover" : ""}" role="button" aria-expanded="${isOpen}"><span class="cl-v">${v.version}</span><div class="cl-main">${isOpen ? "" : `<small>${v.summary}</small>`}</div><span class="cl-d">${date(v.date, lang)}</span>${icon(isOpen ? "chevronDown" : "chevronRight")}</div>`;
        const body = isOpen ? `<div class="cl-body"><div class="cl-sum">${v.summary}</div><ul class="cl-list">${v.bullets.map((b) => `<li>${b}</li>`).join("")}</ul></div>` : "";
        return `<div class="cl-item">${row}${body}</div>`;
      })
      .join("");
    const more =
      shown.length < all.length
        ? `<div class="cl-more">${btn(L.older, { size: "sm", ic: "chevronDown" })}<span class="faint" style="font-size:12px">${L.shown(shown.length, all.length)}</span></div>`
        : `<div class="cl-more"><span class="faint" style="font-size:12px">${L.all(all.length)}</span></div>`;
    const back = `<div class="cl-back">${btn(L.back, { kind: "ghost", size: "sm", ic: "chevronLeft" })}</div>`;
    return `${back}<div class="h1">${L.log}</div><p class="lead">${L.logLead(RUNNING)}</p><div class="grp cl" style="margin-top:18px">${items}${more}</div><div style="height:24px"></div>`;
  }

  // ---- The Markdown source, for the subset decision (page chrome) ------------------
  function source(lang, upTo = 2) {
    const blocks = MD[lang].split(/\n(?=## \[)/).slice(0, upTo + 1).join("\n");
    return esc(blocks)
      .replace(/^(#{1,2} .*)$/gm, '<span class="md-h">$1</span>')
      .replace(/\*\*(.+?)\*\*/g, '<span class="md-b">**$1**</span>');
  }

  function checks() {
    const p = parity(LOG.en, LOG.fr);
    return p ? `Parity: ${esc(p)}` : `<b>Both files parse</b> with the subset parser (${LOG.en.length} versions each) and <b>match</b>: same versions, same bullet counts.`;
  }

  window.Up = { APP, RUNNING, NEXT, icon, toolbar, updateBtn, whatsNew, about, changelog, source, checks, T };
})();
