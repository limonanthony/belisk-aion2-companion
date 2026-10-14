## Belisk, français. Brouillon à relire : aucune de ces phrases
## n'est définitive. Chaque message existe aussi dans en/belisk.ftl,
## avec les mêmes arguments ; un test le vérifie.

## Nom du produit, repris par chaque message qui le cite (la zone de
## notification, les titres de fenêtre) : renommer le produit, c'est
## cette ligne dans chaque langue.

-app-name = Belisk

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
common-time-left = Temps restant
common-character = Personnage
common-server = Serveur
common-skill-numbered = Compétence { $code }
common-unknown-time = --

## Units (sizes take an already formatted value)

unit-bytes = { $value } o
unit-kilobytes = { $value } Ko
unit-megabytes = { $value } Mo
unit-gigabytes = { $value } Go
unit-per-second = /s
unit-damage-billions = { $value }Md
unit-decimal = { $whole },{ $tenth }
unit-percent = { $value } %

## Classes, as the game names them

class-assassin = Assassin
class-chanter = Aède
class-cleric = Clerc
class-elementalist = Spiritualiste
class-fighter = Bagarreur
class-gladiator = Gladiateur
class-ranger = Rôdeur
class-sorcerer = Sorcier
class-templar = Templier

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
date-day-month-year = { $day } { $month } { $year }
date-day-month = { $day } { $month }
list-and = { $rest } et { $last }

## To-dos

todos-section-today = Aujourd'hui
todos-section-this-week = Cette semaine

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
todos-done-of =
    { $done } sur { $total } { $done ->
        [one] faite
       *[other] faites
    }
todos-all-done = tout est fait
todos-done = fait
todos-weekly-on-wednesday = Hebdo le mercredi
todos-not-tracked = non suivi
todos-value-of = { $value } sur { $cap }

todos-resets-in = reset dans { $time }
todos-weekly-reset-when = { $day } · dans { $time }
todos-amount-full-in = Max en { $time }
todos-full-being-wasted = Plein, gaspillé
todos-full-spend-by = À vider avant { $stamp }
todos-full-refills = Plein · rech. { $stamp }

todos-everything-done = Tout est fait sur { $name }.
todos-removed-note = Retiré : { $name }.
todos-added-note = Ajouté à { $section } : { $name }.

todos-source-live = Depuis le jeu, en direct · connecté à { $time }
todos-source-live-now = Depuis le jeu, en direct
todos-source-earlier = Depuis le jeu à la connexion, { $stamp } · recharges depuis estimées
todos-source-earlier-to-dos = Depuis le jeu à la connexion, { $stamp }
todos-source-shared = { $name } pas encore vu à la connexion · lignes Compte de { $from }
todos-source-none = Aucune valeur du jeu pour l'instant. Connectez-vous avec { $name } pendant que { -app-name } tourne.

todos-tip-live = Depuis le jeu, en direct.
todos-tip-extra = +{ $extra } en plus grâce aux objets : jamais rechargés, hors des { $cap }.
todos-tip-extra-entries = +{ $extra } entrées en plus grâce aux objets.
todos-tip-at-login = { $value } à votre connexion, { $stamp }.
todos-tip-game-value = { $value } d'après le jeu, { $stamp }.
todos-tip-refills =
    +{ $added } depuis : { $refills ->
        [one] une recharge
       *[other] { $refills } recharges
    } de { $add }, estimé.
todos-tip-game-at = D'après le jeu, { $stamp }.
todos-tip-restarted = Repartie à zéro au reset hebdomadaire.
todos-tip-shared = D'un autre personnage de ce serveur, { $stamp }.
todos-tip-typed = Saisi par vous. Le jeu indiquait { $value }.
todos-tip-typed-detail = Sa prochaine valeur remplace la vôtre : une dépense, une recharge ou votre prochaine connexion.

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
todos-reset-dialog-body = Sur chaque personnage : toutes les tâches masquées reviennent, et chaque compteur saisi et chaque montant saisi dans la fenêtre des ressources est vidé. Les valeurs du jeu restent.
todos-reset-dialog-body-with-own = Sur chaque personnage : vos propres tâches ({ $own }) sont supprimées, toutes les tâches masquées reviennent, et chaque compteur saisi et chaque montant saisi dans la fenêtre des ressources est vidé. Les valeurs du jeu restent.
todos-reset-dialog-keeps = L'heure de reset et l'abonnement sont conservés. Cette action est irréversible.

todos-window-title = Tâches
todos-window-playing = En jeu
todos-window-picker-note = Les personnages vus par { -app-name }, avec ce qu'il reste à chacun. Les Duty Missions sont partagées par vos personnages d'un même serveur.
todos-window-no-character-title = Aucun personnage
todos-window-no-character-line = Vos tâches apparaissent dès que { -app-name } a vu votre personnage. Lancez-le avant la connexion, ou relog via le choix du perso.
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
todos-window-back-to-game = Revenir à la valeur du jeu : { $label }
todos-window-figure-from-game = { $label }, { $value }, d'après le jeu. Saisir par-dessus
todos-window-figure-typed = { $label }, { $value }, saisi par vous. Saisir par-dessus
todos-window-with-extra = { $value }, { $extra } en plus
todos-window-login-note = Rempli depuis le jeu à la connexion.
todos-window-login-note-replaced = Rempli depuis le jeu à la connexion. Les valeurs saisies ont été remplacées.

resources-window-title = Ressources
resources-window-membership-caps = Plafonds avec abonnement
resources-window-plain-caps = Plafonds sans abonnement
resources-window-no-character-line = Vos ressources apparaissent dès que { -app-name } a vu votre personnage. Lancez-le avant la connexion, ou relog via le choix du perso.
resources-window-picker-note = Les personnages vus par { -app-name }, avec ce qu'il reste à chacun. Les Shugo keys, les Invasion keys et le Daily Dungeon sont partagés par vos personnages d'un même serveur.

## Pets: the window and the soul tracker card

pets-window-title = Familiers
pets-search = Rechercher un familier
pets-clear-search = Effacer la recherche
pets-filter = Afficher et trier
pets-tab-all = Tous
pets-family-cogni = Cogni
pets-family-fera = Fera
pets-family-natura = Natura
pets-family-varian = Varian
pets-family-special = Spécial
pets-owned = { $owned } sur { $total } obtenus
pets-maxed = { $count } au Lv 3
pets-levels = Niveaux
pets-pinned = Épinglés
pets-all-pets = Tous les familiers
pets-no-stats = sans stats
pets-source-live = Du jeu, en direct · connexion à { $time } · { $server }
pets-source-earlier = À votre connexion { $stamp } · reconnectez-vous pour actualiser
pets-source-earlier-tip = Lu à votre connexion { $stamp } ; les âmes gagnées depuis, pendant que { -app-name } tournait, sont ajoutées.
pets-source-earlier-tip-detail = Le jeu n'envoie vos familiers qu'à la connexion : reconnectez-vous pendant que { -app-name } tourne pour des valeurs exactes.
pets-source-none = Aucun familier lu. Connectez-vous pendant que { -app-name } tourne.
pets-souls-per-level = Âmes par niveau
pets-cap = Lv 3 au maximum
pets-level = Lv { $level }
pets-max = Max
pets-pin = Épingler { $name }
pets-unpin = Désépingler { $name }
pets-pin-tip = Épingler en haut
pets-unpin-tip = Désépingler
pets-pinned-announce = { $name } épinglé
pets-unpinned-announce = { $name } désépinglé
pets-show = Afficher
pets-order = Ordre
pets-show-all = Tous les familiers
pets-show-not-owned = Pas encore obtenus
pets-show-not-maxed = Pas au Lv 3
pets-order-game = Ordre du jeu
pets-order-closest = Plus près du niveau suivant
pets-closest-first = les plus proches d'abord
pets-shown = { $count } affichés
pets-clear = Effacer
pets-clear-name = Effacer le choix Afficher et trier
pets-empty-title = Aucun familier pour l'instant
pets-empty-line = Le jeu envoie vos familiers une fois, à la connexion. Connectez-vous avec un personnage pendant que { -app-name } tourne : chaque familier s'affiche ici avec son niveau et ses âmes.
pets-no-match = Aucun familier ne correspond à « { $query } »
pets-nothing-in = Rien dans { $tab }.
pets-search-all = Chercher dans toutes les familles
pets-no-match-all = Aucun familier ne porte ce nom, toutes familles confondues.
pets-unknown = Familier { $id }
pets-row = { $name }, Lv { $level }, { $souls } âmes sur { $needed } vers le Lv { $next }
pets-row-max = { $name }, Lv { $level }, au maximum
pets-card-title = Âmes de familier
pets-card-test = Carte de test
pets-card-reached = Lv { $level } atteint
pets-card-unlocked = Débloqué

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
reminder-detail-dimensional-invasion = Verteron · Altgard · 3 min pour rejoindre
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
reminder-under-way = En cours
reminder-in-minutes = Dans { $minutes } min
reminder-label = Rappel d'événement : { $when }
reminder-start-label = Alerte d'événement : { $when }
reminder-label-event = { $name } : { $detail }
reminder-next = Prochain : { $names } à { $time }
reminder-off = Les rappels d'événements sont désactivés
reminder-none-on = Aucun événement n'est activé

## Settings

settings-end-fight-never = Jamais

settings-window-title = Paramètres de { -app-name }
settings-app-name = { -app-name }
settings-minimise = Réduire
settings-sidebar-title = Paramètres
settings-sections = Sections des paramètres
settings-section-overlay = Overlay
settings-section-behaviour = Comportement
settings-section-capture = Capture
settings-section-bests = Records perso
settings-section-reminders = Rappels
settings-section-todos = Tâches
settings-section-pets = Familiers
settings-section-general = Général
settings-section-about = À propos
settings-meter-status = Compteur : { $status }
settings-status-running = En cours
settings-status-stopped = Arrêté
settings-status-not-running = À l'arrêt
settings-set-aside = Les paramètres enregistrés ont été mis de côté
settings-not-saved = Les paramètres n'ont pas pu être enregistrés
settings-using-defaults = Les paramètres par défaut sont utilisés.

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
settings-overlay-total-damage-text = Sur le tableau Cible. Le tableau Session l'affiche toujours.
settings-overlay-boss-fights = Combats de boss
settings-overlay-fight-summary = Résumé du combat
settings-overlay-fight-summary-text = À la mort d'un boss : votre DPS et votre rang, vos meilleures compétences et votre record perso. Un clic le copie.
settings-overlay-pace = Rythme face à votre record perso
settings-overlay-pace-text = Après 10 s de combat contre un boss, votre avance ou votre retard sur votre record contre ce boss.
settings-overlay-window = Fenêtre
settings-overlay-open-meter = Ouvrir le compteur au lancement
settings-overlay-open-meter-text = Au démarrage de { -app-name }. Désactivé : seule la barre d'outils s'ouvre, et le compteur s'ouvre depuis son icône.
settings-overlay-hide-between-fights = Masquer entre les combats
settings-overlay-hide-between-fights-text = Le compteur s'efface quand vous ne combattez pas et revient à votre premier coup. Son icône dans la barre d'outils l'affiche { $seconds } s.
settings-overlay-lock = Verrouiller l'overlay
settings-overlay-lock-text = Les clics passent au jeu et les commandes se masquent. Alt+Verr. Maj l'active ou le désactive. La barre d'outils prend toujours les clics.
settings-overlay-lock-unavailable = Alt+Verr. Maj n'a pas pu être enregistré : rien ne pourrait déverrouiller l'overlay.
settings-overlay-show-hide = Afficher ou masquer tout
settings-overlay-show-hide-text = Masque la barre d'outils et chaque outil ouvert, puis les fait revenir. « Afficher la barre d'outils » de l'icône de notification les fait revenir aussi.
settings-overlay-show-hide-keys = Alt+Maj+Verr. Maj
settings-key-alt = Alt
settings-key-shift = Maj
settings-key-caps-lock = Verr. Maj
settings-overlay-detail-opens = Ouverture du détail joueur
settings-overlay-detail-opens-text = À côté de l'overlay jusqu'à ce que vous la déplaciez, ou toujours centrée.
settings-overlay-where-left = Là où vous l'avez laissée
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
settings-capture-record-every-session = Enregistrer chaque session
settings-capture-record-every-session-text = Garde le trafic de jeu de cette session dans des fichiers de 30 minutes ; les plus récents sont conservés jusqu'à 2 Go. Les fichiers ne quittent jamais ce PC.
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
settings-capture-rolling-recording = session complète
settings-capture-replaying = Relecture
settings-capture-cancel-replay = Annuler la relecture
settings-capture-replay = Rejouer
settings-capture-no-recordings = Aucun enregistrement
settings-capture-no-recordings-line = Démarrez-en un ci-dessus, ou laissez un serveur inconnu en démarrer un.
settings-capture-replay-failed = La relecture a échoué : { $error }
settings-capture-recording-failed = L'enregistrement a échoué : { $error }
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
settings-reminders-start-alert = Alerte au début
settings-reminders-start-alert-text = Une deuxième carte au moment où l'événement commence.
settings-reminders-sound = Son
settings-reminders-sound-text = Le son « Chimes » de Windows, une fois par carte.
settings-reminders-test-text = Affiche un rappel au-dessus du compteur, avec son son.
settings-reminders-test-shown = Affiché sur le compteur
settings-reminders-show-test = Tester un rappel
settings-reminders-every-hour = Toutes les heures
settings-reminders-every-3-hours = Toutes les 3 heures
settings-reminders-weekly = Chaque semaine
settings-reminders-daily = Chaque jour
settings-event-arena-opens = Ouverture de l'Arena of Tactics
settings-event-shugo-text = À :00. Les mini-jeux, avec 3 min pour rejoindre ; les jeux ouverts ne s'affichent que dans le jeu.
settings-event-invasion-text = À :30. Une des trois invasions, à un endroit au hasard de Verteron ou d'Altgard, avec 3 min pour rejoindre ; vérifiez la carte, elle n'est peut-être pas près de vous.
settings-event-rifts-text = 02:00, 05:00, 08:00, 11:00, 14:00, 17:00, 20:00 et 23:00. Altgard, Verteron, Morheim et Eltnen.
settings-event-kaira-text = 01:00, 04:00, 07:00, 10:00, 13:00, 16:00, 19:00 et 22:00. Boss de terrain dans Chaotic Lower Reshanta.
settings-event-siege-text = Lundi, jeudi et samedi à 21:00. Chaotic Lower Reshanta.
settings-event-abyss-text = Lundi, jeudi et samedi à 21:30. Trois Executors dans Lower Reshanta, trois autres boss dans Middle Reshanta.
settings-event-nahma-text = Vendredi et dimanche à 21:00. Niv. 65, et l'Enragé niv. 80.
settings-event-arena-text = 11:00 et 19:00, à l'ouverture de la file 10 contre 10 (jusqu'à 14:00 et 21:00).
settings-event-keys-text = 16:00. Clés du Shugo Festival et de la Dimensional Invasion.

settings-todos-lead = La fenêtre des tâches, ouverte depuis la barre d'outils. L'heure de reset et l'abonnement règlent aussi les heures de recharge et les plafonds de la fenêtre des ressources.
settings-todos-resets = Resets
settings-todos-daily-reset-text = Aussi l'heure des recharges quotidiennes de la fenêtre des ressources. Signalé, non confirmé : vérifiez le compte à rebours en jeu.
settings-todos-weekly-reset = Reset hebdomadaire
settings-todos-weekly-reset-text = Les tâches de la semaine et les tickets du Daily Dungeon repartent le mercredi, à l'heure du reset quotidien.
settings-todos-weekly-at = Mercredi { $time }
settings-todos-membership = Abonnement
settings-todos-membership-text = Plafonds plus hauts dans la fenêtre des ressources, pour chaque personnage. Odyle Energy 840 au lieu de 560, Shugo keys 21 au lieu de 12.
settings-todos-membership-hint = Le jeu affiche { $amount } sur { $name }, plus que { $cap }. Avez-vous un abonnement ?
settings-todos-membership-amount = { $count } { $label }
settings-todos-defaults = Tâches par défaut
settings-todos-hidden = Tâches masquées
settings-todos-restore = Restaurer les tâches par défaut
settings-todos-start-over = Repartir de zéro
settings-todos-start-over-text = Supprime vos propres tâches, fait revenir toutes celles qui sont masquées et vide chaque compteur saisi et chaque montant saisi dans la fenêtre des ressources.
settings-todos-reset-completely = Tout réinitialiser…
settings-todos-reset-done = Tâches réinitialisées
settings-todos-set-aside = Les tâches enregistrées ont été mises de côté
settings-todos-not-saved = Les tâches n'ont pas pu être enregistrées
settings-todos-starting-empty = Les tâches repartent de zéro.
settings-todos-reset-title = Réinitialiser toutes les tâches ?
settings-todos-reset-confirm = Réinitialiser les tâches

settings-pets-lead = La fenêtre Familiers s'ouvre depuis la barre d'outils. La carte fonctionne compteur ouvert ou fermé.
settings-pets-card = Carte de suivi des âmes
settings-pets-tracker = Suivi des âmes
settings-pets-tracker-text = Une petite carte quand vous ramassez des âmes d'un familier, avec sa progression. Elle disparaît 10 s après le dernier ramassage.
settings-pets-only-pinned = Familiers épinglés seulement
settings-pets-only-pinned-text = N'affiche la carte que pour les familiers épinglés dans la fenêtre Familiers.
settings-pets-maxed = Familiers déjà au Lv 3
settings-pets-maxed-text = Affiche aussi la carte pour les familiers au maximum, en Lv 3 · 75/75.
settings-pets-black-market = Achats au marché noir
settings-pets-black-market-text = Les âmes achetées au marché noir (+5) affichent la carte comme un ramassage.
settings-pets-test-text = Affiche la carte maintenant avec votre premier familier épinglé.
settings-pets-show-test = Afficher une carte de test
settings-pets-test-shown-toolbar = Affichée sous la barre d'outils
settings-pets-test-shown-meter = Affichée au-dessus du compteur
settings-pets-collection = Collection
settings-pets-collection-on = Familiers sur { $server }
settings-pets-collection-no-server = Vos familiers
settings-pets-read = Lue à la connexion, { $stamp }. Le jeu n'envoie vos familiers qu'à la connexion ; les ramassages depuis sont ajoutés.
settings-pets-not-read = Pas encore lue. Connectez-vous avec un personnage de ce serveur pendant que { -app-name } tourne.
settings-pets-not-read-any = Pas encore lue. Connectez-vous avec un personnage pendant que { -app-name } tourne.
settings-pets-owned-of = sur { $total }

settings-general-app = Application
settings-language = Langue
settings-language-text = Les données du jeu (noms des compétences, des boss et des monstres) restent en anglais.
settings-language-system = Système ({ $language })

settings-about-description = Belisk — compteur de combat et compagnon pour AION 2
settings-about-source = Source et mentions
settings-about-licence = Licence : GPL-3.0-only
settings-about-licence-text = Logiciel libre : vous pouvez le partager et le modifier selon la GNU General Public License, version 3.
settings-about-read-licence = Lire la licence
settings-about-credits = Crédits
settings-about-forked-from = Dérivé de
settings-about-upstream = Origine
settings-about-aether = Ce compteur est un fork d'Aether, par Helveticxa. GPL-3.0-only.
settings-about-noia2 = Par zdyoung. Aether, dont ce compteur est un fork, est lui-même un fork de NOIA2 : la chaîne de capture, les analyseurs de paquets, l'overlay et les catalogues de données du jeu en viennent. GPL-3.0-only.
settings-about-kuroukihime = Les noms anglais des monstres et des boss, les noms des compétences du Bagarreur et les codes des serveurs. GPL-3.0.
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
settings-open-no-data-folder = Le dossier des données est inconnu : %APPDATA% n'est pas défini.
settings-open-not-live = Cette exécution ne garde aucun fichier : il n'y a pas de dossier à ouvrir.
settings-open-not-created = { $path } n'a pas pu être créé : { $error }
settings-open-failed = { $target } n'a pas pu être ouvert : { $error }
settings-about-reset = Réinitialiser les paramètres
settings-about-reset-text = Les paramètres reviennent à leurs valeurs par défaut.
settings-about-reset-button = Réinitialiser les paramètres…
settings-reset-title = Réinitialiser les paramètres ?
settings-reset-body = Les paramètres reviennent à leurs valeurs par défaut, y compris les interrupteurs de la page Capture. L'historique des combats, les records perso, les enregistrements et les corrections de boss sont conservés.
settings-about-updates = Mises à jour
settings-about-changelog = Journal des modifications
settings-about-changelog-text = Ce qui a changé à chaque version.
settings-about-open-changelog = Ouvrir le journal
settings-changelog-lead = Ce qui a changé à chaque version. Vous avez la { $version }.
settings-changelog-show-older = Afficher les plus anciennes
settings-changelog-shown = { $shown } versions sur { $total }
settings-changelog-row = { $version }, { $date } : { $summary }
settings-changelog-all = { $total ->
        [one] { $total } version
       *[other] Les { $total } versions
    }
settings-changelog-unreadable = Le journal des modifications n'a pas pu être lu ; belisk.log dit pourquoi.
settings-open-not-web-page = { $target } n'est pas une page web ; rien n'a été ouvert.
settings-updates-note = Demande la dernière version à GitHub ; rien sur vous ni sur votre jeu n'est envoyé.
settings-updates-disabled = Mises à jour désactivées : cette version n'a pas de source d'où les obtenir, ou elle rejoue un enregistrement.
settings-updates-up-to-date = À jour
settings-updates-up-to-date-text = { $version } est la dernière version. Vérifié à { $time } ; vérifié à nouveau au lancement et toutes les heures.
settings-updates-just-checked-text = { $version } est la dernière version. Vérifié à l'instant ; nouvelle vérification possible dans une minute.
settings-updates-checking = Recherche de mises à jour…
settings-updates-checking-text = Demande la dernière version à GitHub.
settings-updates-checking-badge = Recherche
settings-updates-check-now = Vérifier maintenant
settings-updates-available = La version { $version } est disponible
settings-updates-available-text = Publiée le { $date } · { $size }. Ses notes sont sur GitHub jusqu'à la mise à jour ; ensuite, Nouveautés les affiche.
settings-updates-available-undated-text = { $size }. Ses notes sont sur GitHub jusqu'à la mise à jour ; ensuite, Nouveautés les affiche.
settings-updates-release-notes = Notes de version
settings-updates-update-to = Mettre à jour en { $version }
settings-updates-downloading = Téléchargement de { $version }
settings-updates-downloading-text = { $done } sur { $total } · 1 Mo/s au plus, en pause pendant les combats
settings-updates-paused = Téléchargement en pause
settings-updates-paused-text = { $done } sur { $total } · reprend à la fin du combat
settings-updates-progress = Progression du téléchargement
settings-updates-cancel = Annuler
settings-updates-verifying = Vérification du téléchargement
settings-updates-verifying-text = On vérifie que { $version } est complète et authentique.
settings-updates-ready = { $version } est prête à être installée
settings-updates-ready-text = Téléchargée et vérifiée. Redémarrez pour l'installer : l'application se ferme et se rouvre en { $version }, avec vos paramètres, tâches et historique.
settings-updates-restart = Redémarrer pour mettre à jour
settings-updates-try-again = Réessayer
settings-updates-not-checked = Impossible de vérifier les mises à jour
settings-updates-last-checked = Dernière vérification à { $time }.
settings-updates-unreachable = GitHub est injoignable : pas d'internet, ou GitHub est en panne. Prochain essai à { $time }.
settings-updates-rate-limited = GitHub autorise 60 vérifications par heure et par adresse internet, et votre réseau les a épuisées (les autres PC sur la même connexion comptent aussi). Prochaine vérification à { $time }.
settings-updates-auto = Télécharger les mises à jour automatiquement
settings-updates-auto-text = Les nouvelles versions se téléchargent en arrière-plan : débit limité, et en pause pendant les combats. L'installation attend toujours votre clic sur Redémarrer.
settings-updates-failed-hash = Le dernier téléchargement n'a pas passé la vérification (empreinte incorrecte) ; il a été supprimé. Rien n'a été installé.
settings-updates-failed-signature = Le dernier téléchargement n'a pas passé la vérification (signature incorrecte) ; il a été supprimé. Rien n'a été installé.
settings-updates-failed-no-hash = Cette version n'a pas d'empreinte pour vérifier le téléchargement ; rien n'a été installé.
settings-updates-failed-no-signature = Cette version n'a pas de signature pour vérifier le téléchargement ; rien n'a été installé.
settings-updates-failed-download = Le téléchargement s'est arrêté avant la fin ; il a été supprimé. Rien n'a été installé.
settings-updates-failed-unpack = La mise à jour n'a pas pu être décompressée à côté de l'application ; rien n'a été installé.
settings-updates-failed-check = GitHub a répondu, mais pas avec une version que l'application sait lire. La prochaine vérification a lieu dans l'heure.
settings-updates-failed-restart = La nouvelle version n'a pas pu démarrer ; l'application continue de tourner. Rien n'a été modifié.

## Stored files (settings.json, todos.json): the path and the system's or the
## JSON reader's own error come as they are

stored-unreadable = { $file } n'a pas pu être lu : { $error }.
stored-set-aside = { $file } { $problem }. Il a été renommé { $aside }.
stored-not-moved = { $file } { $problem }, et il n'a pas pu être renommé : { $error }.
stored-not-json = n'est pas un JSON valide ({ $error })
stored-not-object = n'est pas un objet JSON
stored-newer = a été écrit par une version plus récente (schéma { $version } ; celle-ci lit { $supported })
stored-wrong-shape = ne peut pas être lu ({ $error })
stored-encode-failed = Les données n'ont pas pu être encodées : { $error }
stored-write-failed = { $file } n'a pas pu être écrit : { $error }

## Meter

meter-report-header = Durée { $duration } · Dégâts totaux { $damage } · DPS du groupe { $dps }
meter-copy-defeated = { $boss }{ $place } · vaincu en { $duration }
meter-copy-ended = { $boss }{ $place } · combat terminé en { $duration }
meter-copy-wiped = { $boss }{ $place } · wipe, boss à { $hp } après { $duration }
meter-copy-rank = #{ $rank } sur { $total }
meter-copy-crit = crit { $percent }
meter-copy-first-record = Premier record
meter-copy-new-best = Nouveau record perso { $pace }
meter-copy-best-behind = Record perso { $best }/s { $pace }
meter-copy-party = Groupe { $dps }/s · { $damage } de dégâts
meter-copy-top = Top : { $skills }
meter-copy-party-list = Groupe : { $players }
meter-copy-player = { $name } · { $damage } · { $dps }/s
meter-copy-you-dps = { $name } { $dps }/s
meter-copy-sign-off = — { -app-name }

meter-status-standing-by = En attente
meter-status-stopped = Arrêté
meter-waiting-title = En attente du premier coup
meter-stopped-title = Le compteur ne tourne pas
meter-capture-no-backend = Aucun pilote de capture disponible ; { $first } ; { $fallback }
meter-capture-driver-error = { $driver } : { $error }
meter-lock-hint = Alt+Verr. Maj verrouille et déverrouille le compteur
meter-unidentified-hint = Lancez avant la connexion, ou relog via le choix du perso
meter-copy = Copier
meter-copied = Copié
meter-reset = Réinitialiser le compteur
meter-capsule = Disposition capsule
meter-lock = Verrouiller le compteur
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

meter-boss-setting = Réglage du boss
meter-close = Fermer le compteur
meter-close-line = Les combats ne sont suivis que tant qu'il est ouvert.
meter-menu-hit-a-mob = Frappez un mob pour indiquer si c'est un boss
meter-menu-treat-as-boss = { $name } est un boss
meter-menu-treat-as-boss-unnamed = C'est un boss
meter-menu-not-a-boss = Pas un boss
meter-menu-automatic = Automatique

meter-summary-defeated = Vaincu
meter-summary-ended = Combat terminé
meter-summary-wiped = Wipe
meter-summary-may-resume = peut reprendre
meter-summary-hp-left = { $hp } restants
meter-summary-party-dps = DPS du groupe
meter-summary-rank = Rang
meter-summary-share = Part
meter-summary-crit = Crit.
meter-summary-party = Groupe
meter-summary-open-report = Ouvrir le rapport de combat
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
history-delete-record = Supprimer cet enregistrement
history-damage = dégâts
history-player-hint = Cliquez sur un joueur pour ouvrir ses détails dans cet enregistrement
history-saved-when = Enregistré quand vous réinitialisez le compteur
history-delete-all = Tout supprimer
history-delete-all-confirm =
    { $count ->
        [one] Supprimer { $count } enregistrement ?
       *[other] Supprimer les { $count } enregistrements ?
    }
history-report = Rapport de combat
history-boss-record = Boss, { $name }
history-wiped-record = Boss, { $name }, wipe
history-wiped-record-hp = Boss, { $name }, wipe avec { $hp } restants

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

## Fight report

report-title = Rapport de combat
report-healing = Soins
report-damage-taken = Dégâts subis
report-deaths = Morts
report-taken = Subis
report-sort-by = Trier par { $column }
report-open-player = { $name }, ouvrir les détails
report-death-label = { $name } est mort à { $time }, tué par { $killer }, { $skill }
report-time = Temps
report-killed-by = Tué par
report-last-hit = Dernier coup
report-by = par
report-nobody-died = Personne n'est mort
report-deaths-not-recorded = Non enregistré pour ce combat
report-not-recorded-note = Les soins, les dégâts subis et les morts n'ont pas été enregistrés pour ce combat : il a été sauvegardé avant que le compteur ne les garde. Les dégâts, les compétences et les buffs sont complets.
report-hint = Cliquez sur un joueur pour ses compétences, buffs, soins et dégâts subis
report-unknown = Inconnu

report-back = Rapport
report-tab-damage = Dégâts
report-tab-buffs = Buffs
report-tab-healing = Soins
report-tab-taken = Dégâts subis
report-multi-hit = Multi-coups
report-multi-hit-damage = Dégâts multi-coups
report-crits = Crit.
report-crit-rate = % crit.
report-heals = Soins
report-healed = Soigné
report-largest = Max
report-monster-skill = Compétence du monstre
report-buff-and-caster = Buff et lanceur
report-up-for = Actif
report-of-fight = sur { $time }
report-on = Sur { $name }
report-died-at = Mort à { $time }
report-last-hit-amount = dernier coup { $amount }
report-no-healing-title = Aucun soin
report-no-healing-line = { $name } n'a lancé aucun soin pendant ce combat.
report-no-taken-title = Aucun dégât subi
report-no-taken-line = Rien n'a touché { $name } pendant ce combat.
report-not-recorded-title = Non enregistré
report-not-recorded-healing = Ce combat a été sauvegardé avant que le compteur ne garde les soins.
report-not-recorded-taken = Ce combat a été sauvegardé avant que le compteur ne garde les dégâts subis.

## PvP

pvp-title = PvP
pvp-watching = Surveillés
pvp-watch-empty = Personne n'est surveillé. Ajoutez un nom, ou faites-vous frapper : votre dernier attaquant apparaît ici de lui-même.
pvp-add-placeholder = Ajouter un joueur à surveiller
pvp-about-capture = Fonctionnement de la capture PvP
pvp-watch-player = Surveiller { $name }
pvp-stop-watching = Ne plus surveiller { $name }
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

## Tray icon

tray-tooltip = { -app-name }
tray-show-toolbar = Afficher la barre d'outils
tray-quit = Quitter

## Toolbar

toolbar-label = { -app-name }
toolbar-grip = Glisser pour déplacer
toolbar-meter = Compteur DPS
toolbar-meter-closed-line = Les combats sont suivis tant qu'il est ouvert.
toolbar-meter-open-line = Cliquer pour le fermer. Les combats ne sont plus suivis.
toolbar-meter-aside-line = Masqué entre les combats. Cliquer pour l'afficher { $seconds } s.
toolbar-pvp = Compteur PvP
toolbar-todos = Tâches
toolbar-pets = Familiers
toolbar-history = Historique des combats
toolbar-reminders = Rappels
toolbar-reminders-paused = En pause
toolbar-settings = Paramètres
toolbar-fold = Replier la barre
toolbar-unfold = Déplier la barre
toolbar-quit = Quitter { -app-name }
toolbar-quit-line = Ferme tous les outils.
toolbar-count-of = { $value } sur { $cap }
toolbar-count-of-extra = { $value } sur { $cap }, +{ $extra } en plus
toolbar-count-full-in = max dans { $time }
toolbar-count-click = Cliquer pour toutes les ressources.
toolbar-count-no-value = Pas encore de valeur. Connectez-vous pendant que { -app-name } tourne.
toolbar-count-label = { $name }, { $value } sur { $cap }. Ouvrir les ressources
toolbar-count-label-extra = { $name }, { $value } sur { $cap }, { $extra } en plus. Ouvrir les ressources
toolbar-count-label-full = { $name }, { $value } sur { $cap }, plein. Ouvrir les ressources
toolbar-count-label-full-extra = { $name }, { $value } sur { $cap }, { $extra } en plus, plein. Ouvrir les ressources
toolbar-count-label-none = { $name }, pas encore de valeur. Ouvrir les ressources
toolbar-more-full = { $count ->
        [one] { $count } autre pleine
       *[other] { $count } autres pleines
    }
toolbar-more-full-label = { $count ->
        [one] { $count } autre ressource pleine
       *[other] { $count } autres ressources pleines
    }
toolbar-folded-full-label = { $count ->
        [one] { $count } ressource pleine. Déplier la barre
       *[other] { $count } ressources pleines. Déplier la barre
    }
toolbar-full-heading = Pleines, une recharge arrive
toolbar-refills = recharge
toolbar-refills-label = { $name }, { $value } sur { $cap }, recharge à { $time }
toolbar-all-resources = Toutes les ressources
toolbar-next-reminders = Prochains rappels
toolbar-no-reminders = Aucun événement activé
toolbar-pause-reminders = Mettre les rappels en pause
toolbar-reminder-settings = Paramètres des rappels
toolbar-update-available = Mise à jour disponible : { $version }
toolbar-update-available-line = Cliquer pour la télécharger. Elle s'installe seulement au redémarrage.
toolbar-update-failed-line = Le dernier téléchargement n'a pas passé la vérification. Cliquer pour réessayer.
toolbar-update-retry-line = Le dernier téléchargement n'a pas abouti. Cliquer pour réessayer.
toolbar-update-downloading = Téléchargement de { $version } · { $percent }
toolbar-update-downloading-line = Débit limité, en pause pendant les combats. Cliquer pour les détails.
toolbar-update-paused = Téléchargement en pause · { $percent }
toolbar-update-paused-line = Reprend à la fin du combat. Cliquer pour les détails.
toolbar-update-verifying = Vérification de { $version }…
toolbar-update-verifying-line = On vérifie que le fichier est complet et authentique.
toolbar-update-ready = Redémarrer pour passer à { $version }
toolbar-update-ready-line = Ferme et rouvre dans la nouvelle version. Rien n'est perdu.
toolbar-update-ready-fight-line = Combat en cours : redémarrer maintenant le coupe en deux dans l'historique.

## First-open prompt

prompt-title = Ouvrir le compteur au lancement ?
prompt-body = { -app-name } démarre avec la barre d'outils seule. Ouvrir le compteur DPS à chaque lancement de { -app-name } ?
prompt-yes = Oui, au lancement
prompt-no = Pas maintenant
prompt-done = Le compteur s'ouvre désormais au lancement.

## What's new, opened once after an update

whatsnew-title = Nouveautés
whatsnew-updated = Mis à jour en { $version }
whatsnew-since = Depuis la { $version }, la dernière version que vous avez lancée
whatsnew-full-changelog = Journal complet
