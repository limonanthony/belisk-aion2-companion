## AION 2 DPS Meter, français. Brouillon à relire : aucune de ces phrases
## n'est définitive. Chaque message existe aussi dans en/aion_2_dps_meter.ftl,
## avec les mêmes arguments ; un test le vérifie.

## Common

common-you = Vous
common-target-numbered = Cible { $id }
common-mob-numbered = Mob { $code }
common-session = Session
common-target = Cible
common-player = Joueur
common-players = Joueurs
common-players-count =
    { $count ->
        [one] { $count } joueur
       *[other] { $count } joueurs
    }
common-damage = Dégâts
common-dps = DPS
common-cancel = Annuler
common-close = Fermer
common-dismiss = Fermer
common-character = Personnage
common-server = Serveur

## Sizes (the value is already formatted)

unit-bytes = { $value } o
unit-kilobytes = { $value } Ko
unit-megabytes = { $value } Mo
unit-gigabytes = { $value } Go

## Durations, dates and lists

duration-days-hours = { $days } j { $hours } h
duration-hours-minutes = { $hours } h { $minutes } min
duration-minutes = { $minutes } min

weekday-short-mon = lun.
weekday-short-tue = mar.
weekday-short-wed = mer.
weekday-short-thu = jeu.
weekday-short-fri = ven.
weekday-short-sat = sam.
weekday-short-sun = dim.

month-short-1 = janv.
month-short-2 = févr.
month-short-3 = mars
month-short-4 = avr.
month-short-5 = mai
month-short-6 = juin
month-short-7 = juil.
month-short-8 = août
month-short-9 = sept.
month-short-10 = oct.
month-short-11 = nov.
month-short-12 = déc.

day-stamp = { $weekday } { $time }
list-and = { $rest } et { $last }

## To-dos

todos-section-today = Aujourd'hui
todos-section-recharges = Recharges
todos-section-this-week = Cette semaine

todos-sub-attempts = tentatives
todos-sub-final-boss = boss final
todos-sub-final-boss-kills = kills du boss final
todos-sub-runs-per-dungeon = 7 runs par donjon

todos-left = { $count } à faire
todos-full =
    { $count ->
        [one] { $count } pleine
       *[other] { $count } pleines
    }
todos-hidden =
    { $count ->
        [one] { $count } masquée
       *[other] { $count } masquées
    }
todos-tracked =
    { $count ->
        [one] { $count } suivi
       *[other] { $count } suivis
    }
todos-done-of =
    { $done } sur { $total } { $done ->
        [one] faite
       *[other] faites
    }
todos-all-done = tout est fait
todos-done = fait
todos-amounts-optional = montants facultatifs
todos-weekly-on-wednesday = Hebdo le mercredi
todos-not-tracked = non suivi
todos-value-of = { $value } sur { $cap }

todos-resets-in = reset dans { $time }
todos-next-full-in = prochain plein dans { $time }
todos-weekly-reset-when = { $day } · dans { $time }
todos-amount-full-in = Max en { $time }
todos-full-being-wasted = Plein, gaspillé
todos-full-spend-by = À vider avant { $stamp }

todos-everything-done = Tout est fait sur { $name }.
todos-amount-full-note = { $label } : plein dans { $time }.
todos-removed-note = Retiré : { $name }.
todos-added-note = Ajouté à { $section } : { $name }.

todos-back =
    { $count ->
        [one] { $count } tâche restaurée
       *[other] { $count } tâches restaurées
    }
todos-restore-keeps = Restaurer les valeurs par défaut conserve vos propres tâches et toute la progression.
todos-hidden-description-none = Rien n'est masqué. { todos-restore-keeps }
todos-hidden-description =
    { $hidden ->
        [one] { $hidden } masquée
       *[other] { $hidden } masquées
    }, sur { $characters ->
        [one] { $characters } personnage
       *[other] { $characters } personnages
    }. { todos-restore-keeps }
todos-reset-dialog-body = Sur chaque personnage : toutes les tâches masquées reviennent, et chaque compteur et chaque montant saisi est vidé.
todos-reset-dialog-body-with-own = Sur chaque personnage : vos propres tâches ({ $own }) sont supprimées, toutes les tâches masquées reviennent, et chaque compteur et chaque montant saisi est vidé.
todos-reset-dialog-keeps = L'heure de reset et l'abonnement sont conservés. Cette action est irréversible.

todos-window-title = Tâches
todos-window-playing = En jeu
todos-window-picker-note = Les personnages vus par le compteur, avec ce qu'il reste à chacun. Les Duty Missions sont partagées par vos personnages d'un même serveur.
todos-window-no-character-title = Aucun personnage
todos-window-no-character-line = Vos tâches apparaissent dès que le compteur a vu votre personnage. Lancez-le avant la connexion, ou relog via le choix du perso.
todos-window-undo = Annuler
todos-window-change = Modifier
todos-window-change-name = Modifier l'heure du reset quotidien dans Paramètres, Tâches
todos-window-daily-reset = Reset quotidien
todos-window-account = Compte
todos-window-yours = Perso
todos-window-hidden-announce = { $label } masquée
todos-window-removed-announce = { $name } supprimée. Annuler se trouve en haut de la fenêtre.
todos-window-put-back-announce = { $label } remise
todos-window-announce-done = { $label } : faite
todos-window-announce-not-done = { $label } : à faire
todos-window-row-done = { $label }, faite
todos-window-row-not-done = { $label }, à faire
todos-window-remove = Supprimer { $label }
todos-window-hide = Masquer { $label }
todos-window-put-back = Remettre { $label }
todos-window-mark-done = Marquer faite
todos-window-mark-done-row = Marquer faite : { $label }
todos-window-undo-row = Annuler : { $label }
todos-window-one-less = Un de moins
todos-window-one-more = Un de plus
todos-window-one-less-row = Un de moins : { $label }
todos-window-one-more-row = Un de plus : { $label }
todos-window-step-less-row = { $step } de moins : { $label }
todos-window-step-more-row = { $step } de plus : { $label }
todos-window-count-done = { $target } sur { $target }, fait
todos-window-show = Afficher
todos-window-show-name = Afficher { $hidden }
todos-window-hide-again = Masquer
todos-window-hide-again-name = Masquer à nouveau : { $hidden }
todos-window-add = Ajouter une tâche
todos-window-new = Nouvelle tâche
todos-window-name = Nom
todos-window-add-button = Ajouter
todos-window-kind = Type
todos-window-tick = Case
todos-window-count = Compte
todos-window-resets = Reset
todos-window-daily = Jour
todos-window-weekly = Semaine
todos-window-how-many = Combien
todos-window-of = sur
todos-window-stop-tracking = Ne plus suivre { $label }
todos-window-amount-input = { $label } : combien vous en avez maintenant

## Event reminders

reminder-event-shugo-festival = Shugo Festival
reminder-event-dimensional-invasion = Dimensional Invasion
reminder-event-rift-portals = Portails de faille
reminder-event-watcher-kaira = Watcher Kaira
reminder-event-artifact-siege = Artifact Siege
reminder-event-abyss-field-bosses = Boss de terrain de l'Abysse
reminder-event-guardian-lord-nahma = Guardian Lord Nahma
reminder-event-arena-of-tactics = Arena of Tactics
reminder-event-reward-keys = Recharge des clés de récompense

reminder-detail-shugo-festival = 3 min pour rejoindre
reminder-detail-dimensional-invasion = Quelque part sur le terrain · 3 min pour rejoindre
reminder-detail-rift-portals = Altgard · Verteron · Morheim · Eltnen
reminder-detail-watcher-kaira = Boss de terrain · Chaotic Lower Reshanta
reminder-detail-artifact-siege = Chaotic Lower Reshanta
reminder-detail-abyss-field-bosses = Lower et Middle Reshanta
reminder-detail-guardian-lord-nahma = Niv. 65, et l'Enragé niv. 80
reminder-detail-arena-of-tactics-early = File 10 contre 10 ouverte jusqu'à 14:00
reminder-detail-arena-of-tactics-late = File 10 contre 10 ouverte jusqu'à 21:00
reminder-detail-reward-keys = Clés du Shugo Festival et de l'Invasion

reminder-test-title = Rappel de test
reminder-test-event = Les rappels ressemblent à ceci
reminder-starting-now = Commence maintenant
reminder-in-minutes = Dans { $minutes } min
reminder-label = Rappel d'événement : { $when }
reminder-label-event = { $name } : { $detail }
reminder-next = Prochain : { $names } à { $time }
reminder-off = Les rappels d'événements sont désactivés
reminder-none-on = Aucun événement n'est activé

## Settings

settings-end-fight-never = Jamais

settings-window-title = Paramètres d'AION 2 DPS Meter
settings-sidebar-title = Paramètres
settings-sections = Sections des paramètres
settings-section-overlay = Overlay
settings-section-behaviour = Comportement
settings-section-capture = Capture
settings-section-bests = Records perso
settings-section-reminders = Rappels
settings-section-todos = Tâches
settings-section-general = Général
settings-section-about = À propos
settings-meter-status = Compteur : { $status }
settings-status-running = En cours
settings-status-stopped = Arrêté
settings-status-not-running = À l'arrêt
settings-set-aside = Les paramètres enregistrés ont été mis de côté
settings-not-saved = Les paramètres n'ont pas pu être enregistrés

settings-overlay-lead = L'apparence de l'overlay du compteur. Les changements s'y affichent aussitôt.
settings-overlay-look = Apparence
settings-overlay-opacity = Opacité du fond
settings-overlay-opacity-text = Plus bas, plus transparent. Le texte garde son propre fond.
settings-overlay-scale = Taille du contenu
settings-overlay-scale-text = De 70 à 150 %.
settings-overlay-layout = Disposition
settings-overlay-layout-text = Capsule tient sur une ligne : votre DPS, votre rang et le temps. Elle s'ouvre au survol.
settings-overlay-layout-full = Complète
settings-overlay-layout-capsule = Capsule
settings-overlay-percentage = Pourcentage
settings-overlay-percentage-text = Contribution porte sur la liste affichée ; Part sur tout le combat.
settings-overlay-contribution = Contribution
settings-overlay-share = Part
settings-overlay-shown = Affiché sur chaque ligne
settings-overlay-boss-hp = Jauge de PV du boss
settings-overlay-player-name = Nom du joueur
settings-overlay-player-name-text = Désactivé, masque les noms ; les icônes et les chiffres restent.
settings-overlay-mask = Masquer les pseudos
settings-overlay-mask-text = Affiche Mi****e au lieu du nom, pour les streams et les captures d'écran.
settings-overlay-combat-power = Puissance de combat
settings-overlay-combat-power-text = À côté du nom, quand le jeu l'a indiquée.
settings-overlay-total-damage = Dégâts totaux
settings-overlay-boss-fights = Combats de boss
settings-overlay-fight-summary = Résumé du combat
settings-overlay-fight-summary-text = À la mort d'un boss : votre DPS et votre rang, vos meilleures compétences et votre record perso. Un clic le copie.
settings-overlay-pace = Rythme face à votre record perso
settings-overlay-pace-text = Après 10 s de combat contre un boss, votre avance ou votre retard sur votre record contre ce boss.
settings-overlay-window = Fenêtre
settings-overlay-lock = Verrouiller l'overlay
settings-overlay-lock-text = Les clics passent au jeu et les commandes se masquent. Alt+Verr. Maj l'active ou le désactive.
settings-overlay-lock-unavailable = Alt+Verr. Maj n'a pas pu être enregistré : rien ne pourrait déverrouiller l'overlay.
settings-overlay-detail-opens = Ouverture du détail joueur
settings-overlay-detail-opens-text = À côté de l'overlay, ou au milieu de l'écran.
settings-overlay-beside = À côté
settings-overlay-centred = Centré
settings-preview = Aperçu
settings-preview-note = Données d'exemple. Suit chaque changement de cette page.

settings-behaviour-lead = Ce que le compteur compte, et ce qu'il fait entre les combats.
settings-behaviour-counted = Ce qui est compté
settings-behaviour-count = Compter
settings-behaviour-count-tip = Avec Boss uniquement et aucun boss en face de vous, chaque coup est ignoré et l'overlay reste vide.
settings-behaviour-count-text = Toutes les cibles enregistre chaque mob frappé ; Boss uniquement ignore tout ce qui n'est pas un boss.
settings-behaviour-all-targets = Toutes les cibles
settings-behaviour-boss-only = Boss uniquement
settings-behaviour-possible-bosses = Détecter les boss possibles
settings-behaviour-possible-bosses-text = Traite comme des boss les monstres à très hauts PV absents de la table des boss. Utile dans les nouveaux donjons.
settings-behaviour-dummy = Mannequin d'entraînement uniquement
settings-behaviour-dummy-text = Ne compte que les dégâts sur votre propre mannequin d'entraînement.
settings-behaviour-hide-unknown = Masquer les joueurs inconnus
settings-behaviour-hide-unknown-text = Écarte les joueurs que le compteur n'a pas pu nommer.
settings-behaviour-players-shown = Joueurs affichés
settings-behaviour-players-shown-text = De 5 à 20.
settings-behaviour-between = Entre les combats
settings-behaviour-end-fight = Terminer un combat après
settings-behaviour-end-fight-text = Sans coup de votre part pendant cette durée, le combat est enregistré dans l'historique et le compteur se vide.

settings-capture-lead = Comment le compteur voit le trafic du jeu. Tout ici est détecté : il n'y a rien à saisir.
settings-capture-connection = Connexion
settings-capture-meter = Compteur
settings-capture-via = via { $backend }
settings-capture-game = Connexion au jeu
settings-capture-found = Trouvée
settings-capture-waiting-game = En attente du jeu
settings-capture-not-identified = Pas encore identifié
settings-capture-backend = Méthode prioritaire
settings-capture-backend-text = La méthode de capture essayée en premier ; l'autre sert de secours. Npcap d'abord : certains anti-triche n'aiment pas qu'un pilote WinDivert soit chargé.
settings-capture-next-start = S'applique au prochain démarrage.
settings-capture-npcap-first = Npcap d'abord
settings-capture-windivert-first = WinDivert d'abord
settings-capture-check = Vérification de la capture
settings-capture-check-text = Droits administrateur, Npcap et WinDivert, comme au démarrage.
settings-capture-checking = Vérification…
settings-capture-rerun = Relancer
settings-check-ready = Tout ce dont le compteur a besoin est en place.
settings-check-blocked = À régler avant que le compteur puisse tourner : { $names }.
settings-check-line = { $label } : { $status }
settings-check-elevation = Droits administrateur
settings-check-elevated = Lancé en administrateur, comme la capture l'exige.
settings-check-not-elevated = Le compteur n'est pas lancé en administrateur : les pilotes de capture ne peuvent pas s'ouvrir. Fermez le compteur et relancez-le.
settings-check-npcap-ok =
    { $count ->
        [one] Installé et opérationnel, { $count } carte réseau visible.
       *[other] Installé et opérationnel, { $count } cartes réseau visibles.
    }
settings-check-available = Disponible.
settings-check-unavailable = Indisponible.
settings-check-windivert-fallback = Fourni en secours. Son pilote n'est chargé que si Npcap ne peut pas capturer, et il est déchargé dès que le compteur n'en a plus besoin.
settings-check-windivert-used = Pilote fourni opérationnel, et utilisé pour la capture.
settings-check-windivert-absent = Absent du dossier du compteur, ce qui ne pose pas de problème tant que Npcap fonctionne. Il ne sert qu'en secours.
settings-check-windivert-missing = Installez Npcap, ou placez WinDivert.dll et WinDivert64.sys à côté du compteur.
settings-capture-recording = Enregistrement
settings-capture-record-unfamiliar = Enregistrer les serveurs inconnus
settings-capture-record-unfamiliar-text = Les deux premières minutes sur un serveur qu'aucun profil ne reconnaît. Les cinq plus récents sont gardés et ne quittent jamais ce PC.
settings-capture-packet-recording = Enregistrement des paquets
settings-capture-packet-recording-text = Écrit les paquets bruts de cette session dans un fichier, pour l'étude du protocole.
settings-capture-recording-progress =
    { $count ->
        [one] { $packets } paquet
       *[other] { $packets } paquets
    } · { $size }
settings-capture-stop-recording = Arrêter l'enregistrement
settings-capture-start-recording = Démarrer l'enregistrement
settings-capture-recordings = Enregistrements
settings-capture-recordings-note = Rejouer en fait passer un dans le compteur en direct, qui doit tourner.
settings-capture-unfamiliar-server = serveur inconnu
settings-capture-replaying = Relecture
settings-capture-cancel-replay = Annuler la relecture
settings-capture-replay = Rejouer
settings-capture-no-recordings = Aucun enregistrement
settings-capture-no-recordings-line = Démarrez-en un ci-dessus, ou laissez un serveur inconnu en démarrer un.
settings-capture-replay-failed = La relecture a échoué : { $error }
settings-capture-replayed =
    { $count ->
        [one] { $packets } paquet rejoué.
       *[other] { $packets } paquets rejoués.
    }
settings-capture-mob-data = Données des mobs
settings-capture-collect-mobs = Collecter les données des mobs
settings-capture-collect-mobs-text = Enregistre chaque mob combattu (code, PV, compteurs) dans mobs.json, pour aider à nommer ceux que la liste fournie ne connaît pas. Aucune donnée de joueur. Les corrections de boss sont enregistrées dans tous les cas.
settings-capture-log = Journal
settings-capture-decision-log = Journal des décisions
settings-capture-decision-log-text = Écrit les décisions du compteur (quel tableau, quel boss, qui vous êtes) dans le fichier journal. Les lignes peuvent contenir des noms de joueurs ; le fichier reste sur ce PC.
settings-capture-meter-log = Journal du compteur
settings-capture-meter-log-text = Pour quand quelque chose semble anormal.
settings-capture-open-log-folder = Ouvrir le dossier des journaux

settings-bests-lead = Votre meilleur DPS par boss et par personnage. Les combats de boss de 15 s ou plus comptent. Conservés quand l'historique est supprimé.
settings-bests-none = Aucun pour l'instant
settings-bests-none-line = Affrontez un boss 15 s ou plus et votre record apparaît ici, par personnage.
settings-bests-count =
    { $bosses } boss, { $characters ->
        [one] { $characters } personnage
       *[other] { $characters } personnages
    }
settings-bests-confirm = Cliquez encore pour réinitialiser
settings-bests-reset-all = Tout réinitialiser
settings-bests-boss = Boss
settings-bests-best-dps = Meilleur DPS
settings-bests-date = Date
settings-bests-row = { $boss }, { $character }, { $dps } par seconde, { $when }

settings-reminders-lead = Un rappel avant les événements à heure fixe, pour arriver à temps. Les heures suivent l'horloge de ce PC et sont réglées pour les serveurs européens, à l'heure de Berlin.
settings-reminders-how = Comment vous êtes prévenu
settings-reminders-master = Rappels d'événements
settings-reminders-master-text = Une petite carte au-dessus du compteur avant chaque événement activé ci-dessous. Elle part seule après 30 s ; un clic la ferme.
settings-reminders-lead-time = Me prévenir
settings-reminders-lead-time-text = Combien de temps avant le début de l'événement.
settings-reminders-minutes-before = { $minutes } min avant
settings-reminders-sound = Son
settings-reminders-sound-text = Le son « Chimes » de Windows, une fois par rappel.
settings-reminders-test-text = Affiche un rappel au-dessus du compteur, avec son son.
settings-reminders-test-shown = Affiché sur le compteur
settings-reminders-show-test = Tester un rappel
settings-reminders-every-hour = Toutes les heures
settings-reminders-every-3-hours = Toutes les 3 heures
settings-reminders-weekly = Chaque semaine
settings-reminders-daily = Chaque jour
settings-event-arena-opens = Ouverture de l'Arena of Tactics
settings-event-shugo-text = À :00. Les mini-jeux, avec 3 min pour rejoindre ; les jeux ouverts ne s'affichent que dans le jeu.
settings-event-invasion-text = À :30. Quelque part sur le terrain, avec 3 min pour rejoindre.
settings-event-rifts-text = 02:00, 05:00, 08:00, 11:00, 14:00, 17:00, 20:00 et 23:00. Altgard, Verteron, Morheim et Eltnen.
settings-event-kaira-text = 01:00, 04:00, 07:00, 10:00, 13:00, 16:00, 19:00 et 22:00. Boss de terrain dans Chaotic Lower Reshanta.
settings-event-siege-text = Lundi, jeudi et samedi à 21:00. Chaotic Lower Reshanta.
settings-event-abyss-text = Lundi, jeudi et samedi à 21:30. Trois Executors dans Lower Reshanta, trois autres boss dans Middle Reshanta.
settings-event-nahma-text = Vendredi et dimanche à 21:00. Niv. 65, et l'Enragé niv. 80.
settings-event-arena-text = 11:00 et 19:00, à l'ouverture de la file 10 contre 10 (jusqu'à 14:00 et 21:00).
settings-event-keys-text = 16:00. Clés du Shugo Festival et de la Dimensional Invasion.

settings-todos-lead = La fenêtre des tâches, ouverte depuis le menu « … » du compteur.
settings-todos-resets = Resets
settings-todos-daily-reset-text = Signalé, non confirmé. Vérifiez le compte à rebours en jeu.
settings-todos-weekly-reset = Reset hebdomadaire
settings-todos-weekly-reset-text = Les tâches de la semaine repartent le mercredi, à l'heure du reset quotidien.
settings-todos-weekly-at = Mercredi { $time }
settings-todos-membership = Abonnement
settings-todos-membership-text = L'Odyle Energy monte jusqu'à 840 au lieu de 560, sur chaque personnage.
settings-todos-defaults = Tâches par défaut
settings-todos-hidden = Tâches masquées
settings-todos-restore = Restaurer les tâches par défaut
settings-todos-start-over = Repartir de zéro
settings-todos-start-over-text = Supprime vos propres tâches, fait revenir toutes celles qui sont masquées et vide chaque compteur et chaque montant.
settings-todos-reset-completely = Tout réinitialiser…
settings-todos-reset-done = Tâches réinitialisées
settings-todos-set-aside = Les tâches enregistrées ont été mises de côté
settings-todos-not-saved = Les tâches n'ont pas pu être enregistrées
settings-todos-reset-title = Réinitialiser toutes les tâches ?
settings-todos-reset-confirm = Réinitialiser les tâches

settings-general-app = Application
settings-language = Langue
settings-language-text = Les données du jeu (noms des compétences, des boss et des monstres) restent en anglais.
settings-language-system = Système ({ $language })

settings-about-description = Un compteur de DPS en temps réel pour AION 2, conçu pour les serveurs internationaux.
settings-about-source = Source et mentions
settings-about-licence = Licence : GPL-3.0-only
settings-about-licence-text = Logiciel libre : vous pouvez le partager et le modifier selon la GNU General Public License, version 3.
settings-about-read-licence = Lire la licence
settings-about-credits = Crédits
settings-about-forked-from = Dérivé de
settings-about-upstream = Origine
settings-about-aether = Ce compteur est un fork d'Aether, par Helveticxa. GPL-3.0-only.
settings-about-noia2 = Par zdyoung. Aether, dont ce compteur est un fork, est lui-même un fork de NOIA2 : la chaîne de capture, les analyseurs de paquets, l'overlay et les catalogues de données du jeu en viennent. GPL-3.0-only.
settings-about-kuroukihime = Les noms anglais des monstres et des boss, les noms des compétences du Fighter et les codes des serveurs. GPL-3.0.
settings-about-references = Références
settings-about-built-with = Réalisé avec
settings-about-built-with-text = GPUI (Apache-2.0) · icônes Lucide (ISC) · Barlow Semi Condensed (SIL OFL 1.1) · WinDivert, le pilote de capture de secours fourni
settings-about-npcap = Non fourni avec le compteur : vous l'installez depuis npcap.com, sous sa propre licence.
settings-about-storage = Stockage
settings-about-stored = Stocké sur ce PC
settings-about-counting = Calcul…
settings-about-storage-line =
    Paramètres { $settings } · Historique des combats { $records ->
        [one] { $records } enregistrement
       *[other] { $records } enregistrements
    }, { $history } · Enregistrements { $files ->
        [one] { $files } fichier
       *[other] { $files } fichiers
    }, { $recordings }
settings-about-open-folder = Ouvrir le dossier
settings-about-reset = Réinitialiser les paramètres
settings-about-reset-text = Les paramètres reviennent à leurs valeurs par défaut.
settings-about-reset-button = Réinitialiser les paramètres…
settings-reset-title = Réinitialiser les paramètres ?
settings-reset-body = Les paramètres reviennent à leurs valeurs par défaut, y compris les interrupteurs de la page Capture. L'historique des combats, les records perso, les enregistrements et les corrections de boss sont conservés.

## Meter

meter-report-header = Durée { $duration } · Dégâts totaux { $damage } · DPS du groupe { $dps }
meter-copy-defeated = { $boss }{ $place } · vaincu en { $duration }
meter-copy-ended = { $boss }{ $place } · combat terminé en { $duration }
meter-copy-rank = #{ $rank } sur { $total }
meter-copy-crit = crit { $percent }
meter-copy-first-record = Premier record
meter-copy-new-best = Nouveau record perso { $pace }
meter-copy-best-behind = Record perso { $best }/s { $pace }
meter-copy-party = Groupe { $dps }/s · { $damage } de dégâts
meter-copy-top = Top : { $skills }
meter-copy-party-list = Groupe : { $players }

meter-status-standing-by = En attente
meter-status-stopped = Arrêté
meter-waiting-title = En attente du premier coup
meter-stopped-title = Le compteur ne tourne pas
meter-lock-hint = Alt+Verr. Maj verrouille et déverrouille le compteur
meter-unidentified-hint = Lancez avant la connexion, ou relog via le choix du perso
meter-copy = Copier
meter-copied = Copié
meter-pace-versus-best = { $pace } vs record
meter-capsule-damage = dégâts
meter-capsule-party-damage = dégâts du groupe
meter-capsule-party-dps = DPS du groupe

meter-switch-tip-target = Afficher le tableau de la cible
meter-switch-tip-target-line = Suit votre cible jusqu'à la réinitialisation ou au prochain boss.
meter-switch-tip-session = Afficher le tableau de la session
meter-switch-tip-session-line = Reste affiché jusqu'à la réinitialisation ou au prochain boss.
meter-switch-tip-pinned-session = Épinglé sur Session
meter-switch-tip-pinned-target = Épinglé sur Cible
meter-switch-tip-unpin = Cliquez à nouveau pour revenir au choix automatique. La réinitialisation et le prochain boss désépinglent aussi.

meter-menu-fight-history = Historique des combats
meter-menu-pvp = Compteur PvP
meter-menu-todos = Tâches
meter-menu-settings = Paramètres
meter-menu-hit-a-mob = Frappez un mob pour indiquer si c'est un boss
meter-menu-treat-as-boss = { $name } est un boss
meter-menu-treat-as-boss-unnamed = C'est un boss
meter-menu-not-a-boss = Pas un boss
meter-menu-automatic = Automatique

meter-summary-defeated = Vaincu
meter-summary-ended = Combat terminé
meter-summary-may-resume = peut reprendre
meter-summary-party-dps = DPS du groupe
meter-summary-rank = Rang
meter-summary-share = Part
meter-summary-crit = Crit.
meter-summary-party = Groupe
meter-best-first = Premier record
meter-best-new = Nouveau record perso
meter-best-behind = Record { $best }

## Fight history

history-unknown-player = Inconnu
history-records =
    { $count ->
        [one] { $count } enregistrement
       *[other] { $count } enregistrements
    }
history-records-filtered =
    { $shown } / { $total } { $total ->
        [one] enregistrement
       *[other] enregistrements
    }
history-deleted =
    { $count ->
        [one] { $count } enregistrement supprimé.
       *[other] { $count } enregistrements supprimés.
    } Les records perso sont conservés.
history-when-today = Aujourd'hui { $time }
history-when-yesterday = Hier { $time }
history-when-weekday = { $weekday } { $time }
history-when-date = { $day } { $month } { $time }

history-title = Historique des combats
history-empty-title = Aucun enregistrement
history-empty-line = Réinitialisez le compteur après un combat pour l'enregistrer ici.
history-no-match = Aucun enregistrement ne correspond aux filtres.
history-all-targets = Toutes les cibles
history-all-players = Tous les joueurs
history-delete-one = Supprimer ?
history-damage = dégâts
history-player-hint = Cliquez sur un joueur pour ouvrir ses détails dans cet enregistrement
history-saved-when = Enregistré quand vous réinitialisez le compteur
history-delete-all = Tout supprimer
history-delete-all-confirm =
    { $count ->
        [one] Supprimer { $count } enregistrement ?
       *[other] Supprimer les { $count } enregistrements ?
    }

## Player detail

detail-no-player = Aucun joueur sélectionné
detail-pick-title = Choisissez un joueur
detail-pick-line = Cliquez sur un joueur dans le compteur, sur une ligne de la fenêtre PvP ou sur un joueur d'un enregistrement de l'historique pour voir ici ses compétences et ses buffs.
detail-no-skills-title = Aucune donnée de compétence
detail-no-skills-live = Ce joueur n'a pas infligé de dégâts à la cible actuelle. Les buffs qu'il a lancés restent affichés ci-dessous.
detail-no-skills-session = Ce joueur n'a infligé aucun dégât depuis la dernière réinitialisation. Les buffs qu'il a lancés restent affichés ci-dessous.
detail-no-skills-history = Ce joueur n'a pas infligé de dégâts à la cible de cet enregistrement. Les buffs qu'il a lancés restent affichés ci-dessous.
detail-chip-history = Historique
detail-chip-live = En direct
detail-no-target = Aucune cible
detail-current-target = cible actuelle
detail-every-target = toutes les cibles depuis la dernière réinitialisation
detail-unknown-target = Cible inconnue
detail-saved-record = enregistrement sauvegardé
detail-saved-record-at = { $time }, enregistrement sauvegardé

detail-skill = Compétence
detail-hits = Coups
detail-crit = Crit.
detail-total = Total
detail-fight = Combat
detail-critical = Critique
detail-back = Dos
detail-front = Face
detail-double = Double
detail-perfect = Parfait
detail-parry = Parade
detail-multi = Multi
detail-multi-damage = Dégâts multi
detail-spec = Spé.
detail-min = Min
detail-max = Max
detail-average = Moyenne
detail-multi-hits = { $hits } coups : { $count } ({ $share })

detail-player-buffs = Buffs du joueur
detail-boss-buffs = Buffs du boss
detail-target-buffs = Buffs de la cible
detail-buffs-up-now =
    { $count ->
        [one] { $count } actif
       *[other] { $count } actifs
    }
detail-buff-up-now = Actif
detail-buff-ended = Terminé

## PvP

pvp-title = PvP
pvp-watching = Surveillés
pvp-watch-empty = Personne n'est surveillé. Ajoutez un nom, ou faites-vous frapper : votre dernier attaquant apparaît ici de lui-même.
pvp-add-placeholder = Ajouter un joueur à surveiller
pvp-seen-this-session = vu cette session
pvp-not-in-range = Hors de portée
pvp-last-attacker = Dernier attaquant
pvp-tab-kills = Stats de kills
pvp-passive-note = La capture est passive : les joueurs hors de portée ne sont pas comptés, et les dégâts sont attribués à la mort de la cible.
pvp-clear = Effacer
pvp-this-session = cette session
pvp-kills-empty-title = Pas encore de stats de kills
pvp-kills-empty-line = Les kills, assistances et morts apparaissent à mesure que les combats se terminent.
pvp-kills = Kills
pvp-assists = Assist.
pvp-deaths = Morts
pvp-damage-dealt = Dégâts infligés
pvp-damage-taken = Dégâts subis
pvp-dealt-empty = Aucun dégât infligé pour l'instant.
pvp-taken-empty = Aucun dégât subi pour l'instant.
