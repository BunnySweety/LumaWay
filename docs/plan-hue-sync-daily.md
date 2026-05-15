# Plan LumaWay — Hue Sync au quotidien (Linux) + trajectoire Musique

Date : 2026-05-15  
Dernière revue : 2026-05-16 (Phase 1.6 — guide première configuration)
Statut : approuvé pour exécution — **document de référence unique** pour la v1.0 écran quotidien, avec trajectoire Musique post-v1.0  
Références : [hue-sync-research.md](hue-sync-research.md), [capture-improvement-roadmap.md](capture-improvement-roadmap.md), [desktop-app.md](desktop-app.md), [backlog.md](backlog.md), [security.md](security.md), [test-matrix.md](test-matrix.md), [architecture-plan.md](architecture-plan.md), [open-questions.md](open-questions.md)

## Comment lire ce plan

| Rôle | Lire en priorité | Puis si besoin |
|------|------------------|----------------|
| **Implémentation** (code) | [§6 Spécification des modes](#6-spécification-des-modes), [§6.1 Priorité des réglages](#61-priorité-des-réglages-tuiles-vs-preset) | [§5 Phases](#5-phases-et-jalons) (phase en cours), [§4 Architecture](#4-architecture-cible), [§7 Structure code](#7-structure-code-suggérée) |
| **QA / validation release** | [§3.4 Critères d’acceptation](#34-critères-dacceptation-comme-simple-que-hue-sync), [§15.2 Critères release v1.0](#152-critères-release-v10) | [test-matrix.md](test-matrix.md), [§15.3 Backlog P0](#p0--avant-ou-avec-la-v10-quotidien) |
| **Produit / UX** | [§1 Vision](#1-vision-produit), [§3 Parité UX](#3-parité-expérience-utilisateur-hue-sync), [§15.1 Décisions](#151-décisions-produit-à-respecter-en-implémentation) | [§3.3 Wireframe](#33-écran-principal-cible-wireframe-texte) |
| **Pilotage** | [§5 Phases](#5-phases-et-jalons), [§9 Ordre d’exécution](#9-ordre-dexécution), [§11 Suivi](#11-suivi) | [§10 Prochaines actions](#10-prochaines-actions), [§15.2 Release](#152-critères-release-v10) |
| **i18n / traduction** | [§3.5 Internationalisation](#35-internationalisation-i18n) | Phase 0 (§5), [§15.6 Revue documentaire](#156-revue-documentaire-alignement-à-limplémentation) |

**Sources de vérité** : modes → §6 + §6.1 ; ordre de livraison → §5 + §9 ; definition of done v1.0 → §15.2 ; modifications du plan → §15.7 + §16.

## Table des matières

1. [Vision produit](#1-vision-produit)
2. [Point de départ](#2-point-de-départ-déjà-en-place)
3. [Parité UX Hue Sync](#3-parité-expérience-utilisateur-hue-sync) — [3.1](#31-principes-ux-non-négociables) · [3.2](#32-écart-gui-actuelle--cible-hue-sync) · [3.3](#33-écran-principal-cible-wireframe-texte) · [3.4](#34-critères-dacceptation-comme-simple-que-hue-sync) · [3.5 i18n](#35-internationalisation-i18n)
4. [Architecture cible](#4-architecture-cible) — [actuelle vs cible](#architecture-actuelle-v10-vs-cible)
5. [Phases et jalons](#5-phases-et-jalons) — [0](#phase-0--fondations-produit-1-semaine) · [1](#phase-1--parité-quotidien-écran--ux-hue-sync-34-semaines) · [2](#phase-2--qualité-perçue-écran-2-semaines-partiellement-en-parallèle) · [3 Musique](#phase-3--musique) · [4](#phase-4--finition-produit-12-semaines)
6. [Spécification des modes](#6-spécification-des-modes) — [6.1 Priorité réglages](#61-priorité-des-réglages-tuiles-vs-preset)
7. [Structure code](#7-structure-code-suggérée)
8. [Risques](#8-risques-et-mitigations)
9. [Ordre d’exécution](#9-ordre-dexécution)
10. [Prochaines actions](#10-prochaines-actions)
11. [Suivi des phases](#11-suivi)
12. [Documentation et index](#12-documentation-et-index)
13. [Éléments reportés](#13-éléments-reportés-hors-phases-immédiates)
14. [Checklist document plan](#14-checklist-de-complétude-du-document-plan)
15. [Lacunes, release, gouvernance](#15-lacunes-identifiées-et-critères-release) — [15.1](#151-décisions-produit-à-respecter-en-implémentation) · [15.2 Release](#152-critères-release-v10) · [15.3 P0/P1/P2](#153-backlog-lacunes-par-priorité) · [15.6](#156-revue-documentaire-alignement-à-limplémentation) · [15.7 Journal](#157-journal-des-corrections-traçabilité)
16. [Gel du plan](#16-gel-du-plan-et-modifications)

---

## 1. Vision produit

**Objectif** : une application GTK utilisable chaque jour sur PC Linux Wayland, avec l’expérience proche de Philips Hue Sync pour l’écran (vidéo, jeux, bureau), puis un mode **Musique** crédible en Phase 3 — **aussi simple à utiliser que l’application officielle** pour un utilisateur non technique, sans prétendre reproduire le binaire Signify ni ses intégrations propriétaires.

**Découpage release** : v1.0 = usage quotidien **écran** (Vidéo / Jeu / Bureau) ; Musique = v1.1/post-v1.0 sauf si la Phase 3 est entièrement terminée avant le gel release.

| Critère de succès | Définition mesurable |
|-------------------|----------------------|
| **Simplicité (parité Hue Sync)** | Après installation, aucune commande terminal requise pour configurer ou utiliser l’app ; zéro jargon visible par défaut (`app-key`, `client-key`, `preset`, chemins `~/.config` masqués). |
| Usage quotidien | Pont + zone configurés une fois ; démarrer la sync en ≤ 2 clics (ou 1 clic depuis la barre système quand l’environnement l’expose). |
| Modes explicites | v1.0 : **Vidéo**, **Jeu**, **Bureau** branchés ; **Musique** visible mais désactivée jusqu’à Phase 3 si elle n’est pas livrée dans la même release. |
| Sync écran | Réaction visible en < 300 ms sur changement de couleur plein écran (validation : **Phase 2.4**, §15.2) ; pas de session « noire » silencieuse. |
| Sync musique | Phase 3 / v1.1 : couleurs qui suivent rythme et énergie sans stroboscope ; choix de la source audio (sortie système par défaut). |
| Robustesse | Arrêt propre (désactivation Entertainment) ; après veille : **Stop propre + message** en v1.0, reconnexion automatique = post-v1.0 ; identité pont via `LUMAWAY_BRIDGE_ID` et pinning optionnel. |
| **Internationalisation** | UI GTK selon la **locale système** (`LANG` / `LC_MESSAGES`) ; anglais (`en`) comme catalogue source et repli ; au minimum **en + fr** en v1.0, extension progressive (de, es, …). |

**Hors scope v1.0**

- Intégrations jeux propriétaires Signify
- Philips Hue Sync TV / Sync Box HDMI
- Builds Windows / macOS
- Mode Musique en v1.0 si Phase 3 n’est pas terminée
- Reverse engineering du binaire Hue Sync (priorité : comparaison comportementale sur motifs connus)

## 2. Point de départ (déjà en place)

Capitaliser sur l’existant :

- Moteur **écran** : XDG Desktop Portal → PipeWire/GStreamer → échantillonnage spatial (`point` / `region`) → Hue Entertainment (DTLS).
- Profils couleur : `soft`, `vivid`, `game`, `boosted`, `cinema`, `desktop`.
- GUI (`lumaway-gui`) : détection pont, pairing, zones, luminosité, réactivité, autostart sync, calibration.
- CLI : `sync`, `capture-quality`, `calibrate-capture`, presets `video-wayland` / `game-wayland` / `desktop-wayland`, alias legacy `tv-wayland`, `doctor`.
- Sécurité Hue : pinning TLS optionnel, `LUMAWAY_BRIDGE_ID`, validation DTLS LAN.

**Manques principaux**

- UX structurée en modes (comme Video / Game / Audio de Hue Sync) — **tuiles Mode et Intensité branchées au démarrage Phase 1 ; i18n accueil/réglages/erreurs principales amorcée ; reste épuration accueil**
- Parité **user-friendly** : trop de réglages techniques visibles (profil capture, `color_profile` anglais, logs techniques)
- Icône barre système et présence en arrière-plan
- Assistant première utilisation
- Mémorisation du flux Portal / multi-écran
- Pipeline **audio** et mode Musique
- **i18n** : gettext et `po/fr.po` amorcés en Phase 0 ; chaînes GUI encore majoritairement à migrer en Phase 1

## 3. Parité expérience utilisateur (Hue Sync)

Objectif : un utilisateur qui connaît Hue Sync sur Windows/macOS doit retrouver **les mêmes gestes** sur Linux, sans lire la doc du dépôt.

### 3.1 Principes UX (non négociables)

1. **Une action principale** : bouton central **Démarrer la synchronisation** / **Arrêter** (état visible, style `suggested-action`).
2. **Modes en premier plan** : Vidéo, Jeu, Bureau, Musique — icône + libellé court ; le mode choisi pilote le moteur (pas un menu « profil couleur » technique). En v1.0, Musique peut être visible mais désactivé avec tooltip jusqu’à Phase 3.
3. **Deux réglages visibles** : curseur **Luminosité** + **Intensité** via les **quatre tuiles** (Subtle → Max) → `LUMAWAY_REACTIVITY`. Choix produit LumaWay : tuiles discrètes plutôt que le curseur continu de Hue Sync desktop — équivalent fonctionnel « intensité / vitesse ». Pas de second curseur Intensité sur l’accueil. Pas de `smoothing` ni `LUMAWAY_PROFILE` exposés par défaut (voir §6.1 pour priorité tuiles vs preset).
4. **Réglages avancés repliés** : pont IP, clés API, profil capture, calibration, pinning TLS, durée, logs détaillés → fenêtre **Réglages** ou section **Avancé**.
5. **Messages humains** : « Appuyez sur le bouton de votre pont Hue », pas `Hue bridge authentication failed (401)`.
6. **État toujours visible** : pont connecté, zone active, sync en cours, capture écran en attente (sélecteur Portal).
7. **Pas de surprise** : la première sync demande la permission écran une fois ; rappel clair si l’utilisateur annule.
8. **Arrière-plan** : minimiser pendant que la sync continue ; si un tray StatusNotifier/AppIndicator est disponible, exposer Start/Stop dans le tray ; sinon garder les mêmes actions dans la fenêtre et limiter les notifications aux erreurs critiques si disponibles. Quitter demande confirmation si sync active.
9. **Langue = locale système** : comme Hue Sync officiel ; pas de langue codée en dur dans l’UI livrée (sauf choix explicite dans Réglages).

### 3.2 Écart GUI actuelle → cible Hue Sync

| Hue Sync (attendu) | LumaWay aujourd’hui | Cible |
|--------------------|---------------------|-------|
| Modes Video / Game / Music (+ bureau) | Tuiles Video/Game/Desktop branchées ; Music visible mais grisée jusqu’à Phase 3 | Finaliser l’UX autour des modes, intensité, i18n complète et états pendant sync |
| Intensité (4 niveaux) | Tuiles Subtle→Max branchées → `LUMAWAY_REACTIVITY`, désactivées pendant sync | Garder comme seul contrôle d’intensité sur l’accueil ; vérifier libellés traduits en Phase 1.4 |
| Luminosité | Curseur Brightness traduit via gettext | Reste validation visuelle multi-locale |
| Pairing en 2 étapes | Boutons Discover / Pair traduits dans Réglages | Assistant traduit : trouver pont → associer |
| Pas de clés visibles | Accueil sans clés ; `app_key` / `client_key` repliées dans Réglages avancés | Remplir automatiquement via association ; assistant traduit à livrer en 1.6 |
| Zone Entertainment | Liste déroulante + switch zone ✓ | « Test lights » traduit ; clarifier zone on/off vs sync (tâche 1.16) |
| Logs techniques | Journal replié dans Réglages | Garder replié par défaut ; messages humains en premier plan |
| Profil `vivid` / `game`… | Menu déroulant anglais technique | Déduit du **mode** ; avancé seulement |
| `Capture profile` / Calibrate | Repliés dans Réglages avancés | Garder en Avancé + proposition auto si échec |
| Tray | Absent | Phase 1.7 si support StatusNotifier/AppIndicator détecté ; fallback fenêtre Start/Stop obligatoire + notification minimale si disponible |
| Langue UI | Gettext initialisé ; accueil/réglages/statuts principaux + erreurs courantes migrés | Revue visuelle locale + complétion au fil des écrans restants |
| Langue ≠ OS | N/A | Optionnel : `LUMAWAY_LANG` ou sélecteur dans Réglages |

### 3.3 Écran principal cible (wireframe texte)

Les libellés ci-dessous sont des **clés sémantiques** ; l’UI affiche la traduction selon `LC_MESSAGES` (ex. `fr_FR.UTF-8` → français).

```text
┌─ LumaWay ───────────────────────────── [Settings] ─┐
│  ● Connected — Living room (Hue bridge)           │
├───────────────────────────────────────────────────┤
│  Mode                                             │
│  [Video] [Game] [Desktop] [Music]                 │
│  Intensity:  [ Subtle ] [ Moderate ] [ High ] [ Max ]│
├───────────────────────────────────────────────────┤
│  Zone: [ TV ▼ ]              [Test lights]        │
│  Brightness  ████████░░                           │
├───────────────────────────────────────────────────┤
│         [  Start sync  ]  /  [ Stop sync  ]        │
│  Status: Ready / Syncing… / Pick a screen         │
└───────────────────────────────────────────────────┘
```

Réglages (secondaire) : pont, associer, langue (optionnel), source audio quand Musique est livré, autostart, journal technique, calibration, sécurité.

### 3.4 Critères d’acceptation « aussi simple que Hue Sync »

Test manuel avec un **utilisateur non développeur** (checklist) :

- [ ] Installation via `install-desktop-app.sh` puis lancement menu — sans lire le README.
- [ ] Premier lancement : assistant jusqu’à sync allumée en **< 15 min** sans terminal.
- [ ] Aucun champ `app-key` / `client-key` / `profile` sur l’écran principal.
- [ ] Après **Stop**, passer Vidéo → Jeu puis redémarrer la sync : réactivité nettement plus forte **sans** ouvrir Réglages (cohérent tâche 1.15 — pas de changement de mode à chaud en v1.0).
- [ ] Message clair si le bouton pont n’a pas été pressé avant Associer.
- [ ] Message clair si le sélecteur Portal est annulé ou ne renvoie pas d’image.
- [ ] Arrêt : lumières Entertainment désactivées, statut « Arrêté ».
- [ ] Tray : démarrer/arrêter sans rouvrir la fenêtre **si** le bureau expose StatusNotifier/AppIndicator ; sinon fenêtre Start/Stop accessible + notification minimale d’erreur critique si disponible.
- [ ] Mode Musique (quand livré) : démarrage sans demande de capture écran.
- [ ] `LANG=fr_FR.UTF-8` : toute l’UI principale en français (pas de chaîne EN résiduelle sur l’accueil).
- [ ] `LANG=C` ou locale non traduite : repli **anglais** lisible (pas de clés gettext brutes type `mode.video.label`).
- [ ] `.desktop` et métadonnées AppStream : nom/description traduits pour les langues livrées.

### 3.5 Internationalisation (i18n)

Objectif : même niveau que Hue Sync officiel — l’app suit la langue du bureau Linux (GNOME/KDE), avec catalogues maintenables par des contributeurs.

#### Stack recommandée (GTK / Rust)

| Composant | Choix |
|-----------|--------|
| Format | **GNU gettext** (`.pot` / `.po` / `.mo`) — standard GNOME et Flatpak |
| Crate v1.0 | [`gettext-rs`](https://docs.rs/gettext-rs/) (crate Rust `gettextrs`) : `gettext`, `ngettext`, `bindtextdomain`, `textdomain` ; helper local `tr()` / `tr_format()` pour garder les appels UI lisibles ; `.mo` installés par `install-desktop-app.sh` (standard GNOME) |
| Crate post-v1.0 | `i18n-embed` seulement si Flatpak/snap doit tourner **sans** fichiers locale système |
| Catalogue source | **`en`** — toutes les chaînes UI en anglais dans le code (msgid) |
| Répertoire | `po/` à la racine du dépôt ; `LINGUAS` liste les langues actives |
| Install | `scripts/install-desktop-app.sh` compile `msgfmt` → `~/.local/share/locale/.../LC_MESSAGES/lumaway-gui.mo` ; installe aussi `.desktop` et AppStream `.metainfo.xml` traduits |
| Flatpak | inclure les `.mo` dans le manifest (Phase 4) |

#### Périmètre traduit (par priorité)

| Périmètre | v1.0 | Post-v1.0 |
|-----------|----|---------|
| `lumaway-gui` (accueil, réglages, assistant, tray, erreurs utilisateur) | **Oui** | — |
| Fichier `.desktop` / AppStream | **Oui** (en + fr minimum) | Autres langues |
| Messages d’erreur mappés (`user_messages`) | **Oui** (codes stables → gettext) | — |
| CLI `lumaway` (sortie stderr interactive) | Anglais + codes | gettext si `TEXTDOMAIN=lumaway` |
| Logs techniques / `sync_stats` | **Non** (anglais, pour le support) | — |
| Noms de zones Hue | **Non** (données pont) | — |
| Valeurs config (`LUMAWAY_*`) | **Non** (identifiants techniques) | — |

#### Locales cibles

| Priorité | Code | Remarque |
|----------|------|----------|
| P0 | `en` | Source + fallback |
| P0 | `fr` | Première locale non anglaise (validateur courant) |
| P1 | `de`, `es`, `nl`, `it` | Aligné sur les langues courantes Hue Sync / Signify |
| P2 | `pt_BR`, `pl`, `sv`, `da`, `nb`, `fi`, `cs`, `hu`, `tr`, `ja`, `zh_CN` | Selon contributeurs ; pas bloquant v1.0 |

#### Règles de traduction

- **Pluriels** : utiliser `ngettext` pour compteurs (ex. « %d lights »).
- **Placeholders** : `tr_format("Connected to {name}", &[("name", name)])` ou helper équivalent — ne pas concaténer des chaînes traduites.
- **Modes** : msgid courts et stables (`Video`, `Game`, `Desktop`, `Music`) ; pas de noms de preset technique visibles.
- **RTL** : pas requis en v1.0 (langues Hue Sync actuelles) ; GTK gère l’essentiel si ajout `ar` plus tard.
- **Override** : `LUMAWAY_LANG=fr_FR` (option Réglages) si la locale système ne convient pas ; sinon `setlocale` + `bindtextdomain` au démarrage.

#### CI / qualité

- Cible `msgfmt --check` sur tous les `po/*.po`.
- Test manuel : lancer GUI avec `LANG=fr_FR.UTF-8` et `LANG=de_DE.UTF-8`.
- Éviter les chaînes dans le CSS ou les icônes seules sans tooltip traduit.

#### Phase 0 — amorçage i18n (avec les presets)

- Ajouter dépendance `gettext-rs`, initialiser `setlocale`, `bindtextdomain("lumaway-gui", ...)` et `textdomain("lumaway-gui")` dans `main()`.
- Créer `po/POTFILES.in`, `po/LINGUAS`, premier `po/fr.po` (même partiel).
- Remplacer progressivement les littéraux dans `main.rs` par `tr(...)` / `tr_format(...)`.

## 4. Architecture cible

```text
lumaway-gui
  ├── Modes (Vidéo / Jeu / Bureau / Musique)
  ├── Tray (Start / Stop / état)
  └── Assistant première utilisation
           │
           ▼
SyncOrchestrator (lumaway-cli + lumaway-core)
  ├── ScreenSource  → Portal / PipeWire / GStreamer
  ├── AudioSource   → PipeWire ou Pulse (mode Musique)
  ├── ColorPipeline → profil + lissage + grading
  └── HueStream + DTLS → pont
```

**Principe** : une seule source active par session (**écran** ou **audio**), un seul flux Entertainment actif, même encodeur HueStream et transport DTLS. Ne pas lancer capture écran et analyse audio en parallèle sur le même pont.

<a id="architecture-actuelle-v10-vs-cible"></a>
### Architecture actuelle (v1.0) vs cible

| Aspect | Aujourd’hui (code) | Cible (plan) |
|--------|-------------------|--------------|
| GUI → moteur | `lumaway-gui` lance **`lumaway sync`** (ou futur `audio-sync`) en **sous-processus** (`Child`), variables via `lumaway.env` + env au Start | `SyncOrchestrator` partagé in-process (`lumaway-core` + lib CLI) — **post-v1.0 ou refactor** |
| Preset au Start | `DEFAULT_SYNC_MODE=video` + preset dérivé `video-wayland` ; tuiles Mode non branchées | `LUMAWAY_SYNC_MODE` résout le preset (`video-wayland`, etc.) |
| Erreurs | Parsing stderr / logs du subprocess | Codes stables → `user_messages` + gettext |

La v1.0 peut **rester en subprocess** tant que la GUI propage correctement `LUMAWAY_SYNC_MODE`, le preset dérivé et gère la **sortie inattendue** du CLI (voir tâches 1.3, 1.13).

## 5. Phases et jalons

### Phase 0 — Fondations produit (~1 semaine)

**But** : verrouiller le contrat « mode » avant les gros chantiers UI et audio.

| Livrable | Détail |
|----------|--------|
| Spécification modes | Table §6 + règles §6.1 ; `SyncMode` dans `lumaway-core`. |
| Presets CLI | `video-wayland`, `game-wayland`, `desktop-wayland` (affinage de `tv-wayland` existant). |
| Alias `tv-wayland` | Conserver `tv-wayland` comme alias de `video-wayland` pour ne pas casser GUI, profils et scripts existants. |
| Alias `LUMAWAY_PRESET` | Si `LUMAWAY_PRESET=tv-wayland` sans `LUMAWAY_SYNC_MODE`, migration → `video` + log une fois. |
| `SyncMode` Rust | Enum dans **`lumaway-core`** (`sync_mode.rs`) ; importé par CLI et GUI (pas de duplication de chaînes) ; résolution preset + `LUMAWAY_COLOR_PROFILE` par défaut. |
| Tests d’acceptation | Motifs écran documentés : plein écran RGB, split gauche/droite, fenêtre mobile, contenu sombre — voir [capture-improvement-roadmap.md](capture-improvement-roadmap.md). |
| Doc opérateur | Section « premier lancement » : pont → bouton → zone → test rouge → sync. |
| Amorçage **i18n** | `po/`, gettext dans `lumaway-gui`, `en` + squelette `fr` (voir §3.5). |
| Métadonnées desktop / AppStream | Créer `packaging/desktop/io.github.BunnySweety.LumaWay.metainfo.xml.in`, inclure `.desktop.in` + `.metainfo.xml.in` dans `po/POTFILES.in`, installer les versions traduites via `install-desktop-app.sh`. |
| **Config version** | Champ `LUMAWAY_CONFIG_VERSION=1` ; migration si ancien `lumaway.env` sans `LUMAWAY_SYNC_MODE`. |

**Critère de fin** : `lumaway sync --preset game-wayland` et `--preset video-wayland` produisent un comportement distinct mesurable (réactivité, saturation) ; `msgfmt` produit un `.mo` fr sans erreur ; `.desktop` et AppStream traduits existent pour `en` + `fr`.

### Phase 1 — Parité « quotidien écran » + UX Hue Sync (~3–4 semaines)

**But** : ressentir Hue Sync Vidéo / Jeu / Bureau **et** une interface aussi accessible que l’app officielle.

| # | Fonctionnalité | Travail technique |
|---|----------------|-------------------|
| 1.1 | **Câbler les tuiles Mode** | Remplacer **Scenes** par **Desktop** dans `mode_row()` ; icône Desktop ex. `computer-symbolic` (ne pas réutiliser `applications-graphics-symbolic` de Scenes) ; Video / Game / Desktop / Music → `LUMAWAY_SYNC_MODE` ; Musique grisée jusqu’à Phase 3 avec tooltip traduit (« Coming soon » / « Bientôt »). |
| 1.2 | **Câbler l’intensité** | Fait : `preset_row()` → `LUMAWAY_REACTIVITY` (§6.1), accueil sans curseur « Reactivity », tuiles **insensibles pendant sync**, défaut Jeu = High au premier run sans valeur sauvegardée. |
| 1.3 | Mapping modes → moteur | Voir §6–§6.1 ; au **Start** : `LUMAWAY_SYNC_MODE`, preset dérivé, `LUMAWAY_COLOR_PROFILE` imposé par le mode (écrase la valeur Réglages sauf mode « avancé » futur) ; ne plus utiliser `DEFAULT_PRESET` en dur. |
| 1.4 | **Chaînes i18n complètes** | Socle livré : libellés accueil + réglages + statuts principaux via `tr(...)` / `tr_format(...)`, `po/fr.po` étendu, erreurs courantes via codes stables + `user_messages` ; reste revue visuelle locale et extension au fil des écrans ajoutés. |
| 1.5 | **Écran principal épuré** | Fait : accueil limité aux gestes quotidiens ; `duration`, `profile`, `color_profile`, clés, réactivité fine, actions Quality/Calibrate et journal repliés dans Réglages avancés ; option **cinema** uniquement en Réglages avancé (§6). |
| 1.6 | Assistant première utilisation | Partiel livré : carte accueil “First setup” tant que la configuration est incomplète, avec étapes pont → bouton physique / Pair → zone → Test lights → Start sync ; si le pont ne renvoie aucune zone Entertainment, état guidé pour créer une zone dans l’app Hue puis recharger. Assistant pleine page post-v1.0 si nécessaire. |
| 1.7 | Icône barre système | StatusNotifier/AppIndicator : état, Start/Stop, mode, quitter (confirmation si sync) **quand le bureau le supporte**. Sur GNOME vanilla sans extension tray, fallback requis : fenêtre qui conserve Start/Stop + notification minimale pour erreurs critiques si le portail/serveur de notifications est disponible. |
| 1.8 | Démarrage de session | Fait : option Réglages traduite pour ouvrir LumaWay à la connexion via `~/.config/autostart/io.github.BunnySweety.LumaWay.desktop`; option séparée « Start sync when app opens » conservée pour lancer la sync à l’ouverture. |
| 1.9 | Échecs explicites | Fait : erreurs Portal / capture / pont classifiées via gettext et complétées par actions contextuelles `Retry` et/ou `Open Settings` sur l’accueil. |
| 1.10 | Flux Portal | Fait : statut traduit « Choose the screen or window to sync » pendant l’ouverture du sélecteur ; `lumaway sync` réutilise et persiste `LUMAWAY_PORTAL_RESTORE_TOKEN` si le portail renvoie un `restore_token`, sinon le rappel reste affiché à chaque session. |
| 1.11 | **Bouton unique sync** | Fait : bouton unique Start sync / Stop sync ; Réglages, pairing, découverte, zone, luminosité, intensité et champs avancés bloqués pendant sync. |
| 1.12 | **Langue (optionnel v1.0)** | Sélecteur langue dans Réglages ou `LUMAWAY_LANG` ; sinon locale OS uniquement. |
| 1.13 | **Robustesse quotidienne** | P0 livré côté code : **sortie inattendue du subprocess `lumaway`** → UI + message i18n + Start disponible ; le dernier log d’erreur classe pont / Portal / capture / DTLS via `user_messages` ; flux Portal sans nouvelle frame > 5 s → erreur classifiée + arrêt/désactivation Entertainment ; échec d’envoi DTLS pendant sync → message “connexion pont perdue” + Start disponible ; reprise après veille détectée par écart horloge murale / monotone > 5 s → Stop + message i18n. Reste validation manuelle §15.3 ; reconnexion DTLS continue = post-v1.0. |
| 1.14 | **Instance unique GUI** | Fait : `application_id` GTK unique ; une activation secondaire présente la fenêtre existante au lieu de reconstruire une seconde fenêtre/sync. |
| 1.15 | **Changement de mode** | Fait : en v1.0, changement de mode **après Stop** ; tuiles Mode désactivées pendant sync avec tooltip traduit ; bascule à chaud = post-v1.0. |
| 1.16 | **Interrupteur zone** | Fait : tooltip traduit ; le switch **zone on/off** (`area_enabled`) contrôle la zone Hue via l’API pont, **distinct** de Start/Stop sync. |

**Critère de fin** : checklist §3.4, §3.5 et critères release §15.2 sur **GNOME Wayland** ; Start/Stop ≤ 2 actions depuis la fenêtre, et depuis le tray quand disponible ; notification limitée aux erreurs critiques si disponible ; pas de terminal après installation.

### Phase 2 — Qualité perçue écran (~2 semaines, partiellement en parallèle)

**But** : moins de calibration manuelle pour un résultat « ça marche » comparable à Hue Sync.

| # | Livrable |
|---|----------|
| 2.1 | Profils de crop persistants par écran (`profiles/<name>-crop.env` ou clés dédiées). |
| 2.2 | Courbes par défaut plus lumineuses en mode Vidéo sur contenu sombre (anti-noir doux). |
| 2.3 | `backend-probe` proposé dans l’assistant si première sync noire. |
| 2.4 | Harness de comparaison documenté (motifs fixes → **latence perceptible &lt; 300 ms** sur changement plein écran, couleur par canal) ; seuil documenté dans le harness ou la checklist release. |
| 2.5 | Réutiliser `lumaway sample-debug` et `capture-quality` dans le flux diagnostic (déjà implémentés). |
| 2.6 | Placement 3D Entertainment (`position.z`) — **post-v1.0** si le pont expose des coordonnées exploitables. |

**Critère de fin** : utilisateur TV existant sans `calibrate-capture` obligatoire ; nouvel utilisateur ≤ 10 minutes jusqu’à une sync satisfaisante.

<a id="phase-3--musique"></a>
### Phase 3 — Musique (~3–4 semaines, v1.1 si non livrée avant v1.0)

**But** : mode Musique utilisable en session longue, pas un prototype FFT instable.

#### Musique — spike technique (semaine 1)

| Tâche | Choix recommandé |
|-------|------------------|
| Capture audio Linux | **PipeWire** en priorité ; repli **PulseAudio** (`parec` / libpulse). |
| Format | PCM stéréo 48 kHz, buffers 20–50 ms. |
| Analyse | FFT → bandes basse / médiums / aigus + RMS global. |
| Sortie lumière initiale | Une couleur **globale** pour toute la zone Entertainment. |
| Cadence | Analyse 30–60 Hz → stream Hue 25 Hz (répétition de frame comme pour l’écran). |

Commande spike :

```text
lumaway audio-sync --bridge <ip> --area <id> --duration-ms 0 --audio-source default
```

Réutilise activation Entertainment + DTLS existants ; **pas** de capture écran.

#### Musique — algorithme couleur (v1.1 initiale)

```text
bandes normalisées (attaque / release sur l’énergie)
teinte ← rapport basse / aigus (ex. basse → chaud, aigu → froid)
saturation ← énergie globale
lissage temporel fort (anti-stroboscope)
silence prolongé → extinction douce ou couleur tamisée
```

- Nouveau profil : `ColorProfile::Music` (gain / gamma / saturation dédiés).
- Préréglages v1.1 : **Ambiant** (lissage fort) vs **Fête** (réactivité haute).
- Détection de beat simple (pic basse + cooldown) en v1.1.

#### Musique — intégration produit

| Livrable | Détail |
|----------|--------|
| Mode GUI Musique | Start lance `audio-sync` ; Stop identique à l’écran. |
| Choix source audio | Liste des sorties PipeWire/Pulse dans Réglages ; défaut = monitor de la sortie système. |
| Variables | `LUMAWAY_AUDIO_SOURCE`, `LUMAWAY_SYNC_MODE=music`. |
| Flatpak (futur) | Permission audio documentée. |

**Critère de fin** : 30 minutes d’écoute variée sans arrêt manuel ; fin audio → couleur neutre ou extinction en < 5 s.

#### Musique — validation audio

- Pistes de référence documentées (ou sous `docs/fixtures/audio/`) : silence, basse continue, voix, électro à BPM stable.
- Scénarios ajoutés dans [test-matrix.md](test-matrix.md) : source audio indisponible, changement de sortie pendant la sync, pause prolongée.
- Comparaison comportementale optionnelle avec Hue Sync sur une autre machine (même morceau, observation visuelle) — pas de RE binaire.

#### Musique — extension v2 (optionnelle)

- Mapping **spatial audio** : basses → canaux « bas », aigus → canaux « haut » selon `position.y` Entertainment.
- Préréglages musique persistés dans `profiles/music.env`.

### Phase 4 — Finition produit (~1–2 semaines)

| Livrable | Détail |
|----------|--------|
| `lumaway doctor` | PipeWire/Pulse, GStreamer, source audio par défaut. |
| README | Section « Comparaison Hue Sync » + guide traduction (`xgettext`, `po/`). |
| Flatpak | Permissions Portal + audio ; embarquer tous les `.mo` de `LINGUAS`. |
| AppStream | Inclure `io.github.BunnySweety.LumaWay.metainfo.xml` traduit dans le paquet natif/Flatpak ; validation `appstreamcli validate` si disponible. |
| Locales P1 | `de`, `es` (et autres selon contributeurs). |
| Raccourcis globaux | Toggle sync (optionnel, post-v1.0). |
| Reprise DTLS | **v1.0** : au plus **3 tentatives au total** de handshake initial + message i18n si échec ; reconnexion continue / backoff = post-v1.0 ([open-questions.md](open-questions.md)). |
| [test-matrix.md](test-matrix.md) | Modes, tray, assistant, Flatpak, **i18n**, robustesse (§15). |
| CI | `msgfmt --check` sur `po/*.po` dans le pipeline (Phase 0 / 4). |
| Guide traduction | Section README ou `CONTRIBUTING.md` : `xgettext`, éditer `po/*.po`, `msgfmt` (renvoie §3.5). |
| Migration config | Déjà Phase 0 (`LUMAWAY_CONFIG_VERSION`) — pas de doublon ici. |

## 6. Spécification des modes

**Statut** : figé pour l’implémentation Phase 0 ; seules les valeurs marquées « à valider en test » peuvent bouger après bench hardware.

**Décision Scenes** : la tuile « Scenes » du mockup GUI n’est **pas** un mode Hue Sync ; elle est remplacée par **Desktop** (`LUMAWAY_SYNC_MODE=desktop`). Pas de cinquième mode en v1.0.

| Mode | `LUMAWAY_SYNC_MODE` | Source | ColorProfile | capture_fps | smoothing preset | Notes |
|------|---------------------|--------|--------------|-------------|------------------|-------|
| Vidéo | `video` | Écran | **`vivid`** (défaut) ; voir **cinema avancé v1.0** ci-dessous | 8 | 0.35 | `region`, auto-crop optionnel CLI |
| Jeu | `game` | Écran | `game` | **12** (fixé Phase 0 ; bench peut ajuster ±2) | 0.65–0.85 | `max_step` désactivé ; tuile intensité par défaut High ou Max (§1.2) |
| Bureau | `desktop` | Écran | `desktop` | 6 | 0.50 | priorité CPU / stabilité |
| Musique | `music` | Audio | `music` (nouveau) | — | **0.30** (défaut interne ; tuiles écrasent au Start) | pas de Portal ; `audio-sync` uniquement |

Preset CLI associés :

| Mode | Preset |
|------|--------|
| Vidéo | `video-wayland` (évolution de `tv-wayland`) |
| Jeu | `game-wayland` |
| Bureau | `desktop-wayland` |
| Musique | pas de preset capture ; flags `audio-sync` |

Persistance dans `~/.config/lumaway/lumaway.env` :

```text
LUMAWAY_CONFIG_VERSION=1
LUMAWAY_SYNC_MODE=video
LUMAWAY_AUDIO_SOURCE=default
LUMAWAY_MUSIC_STYLE=ambient   # v1.1 : ambient | party
# Déprécié : ne plus éditer à la main — dérivé du mode au Start
# LUMAWAY_PRESET=video-wayland
```

**`LUMAWAY_PRESET` vs `LUMAWAY_SYNC_MODE`** : à partir de la v1.0 planifiée, **`LUMAWAY_SYNC_MODE` est la source de vérité** (Video / Game / Desktop / Music). Le preset CLI (`video-wayland`, `game-wayland`, …) est **calculé au Start** et passé au subprocess. `LUMAWAY_PRESET` reste accepté en **lecture seule / alias** pour compatibilité (`tv-wayland` → mode Video) puis disparaît de la doc utilisateur.

**Tuiles d’intensité → `LUMAWAY_REACTIVITY`** (valeurs cibles à valider en test) :

| Tuile (msgid) | `LUMAWAY_REACTIVITY` | Effet |
|---------------|----------------------|--------|
| Subtle | `0.20` | Lissage fort |
| Moderate | `0.35` | Défaut actuel / équilibré |
| High | `0.65` | Réactif |
| Max | `0.90` | Jeu / réaction rapide |

**Mode Vidéo — profil `cinema` en Réglages avancé v1.0** : case ou liste « Profil cinéma » (traduite) écrit `LUMAWAY_COLOR_PROFILE=cinema` dans `lumaway.env`. Au prochain **Start** en mode Video, si `cinema` est activé, il **remplace** le défaut `vivid` ; sinon le mode impose `vivid`. Hors mode Video, ce réglage n’est pas appliqué. Les autres overrides avancés de `LUMAWAY_COLOR_PROFILE` restent post-v1.0.

**Mode Musique — intensité et lissage** : les tuiles Subtle→Max écrivent `LUMAWAY_REACTIVITY`, passé à `audio-sync` comme **poids de réaction** sur l’analyse FFT (équivalent de `--smoothing` pour l’écran). Le `smoothing` preset musique (0.30 par défaut Phase 0) est le **défaut interne** si aucune tuile n’a encore été choisie ; dès qu’une tuile est active, **elle écrase** ce défaut au Start (même règle que §6.1). Le style **Ambiant / Fête** (`LUMAWAY_MUSIC_STYLE`, v1.1) module l’algorithme, pas les tuiles.

### 6.1 Priorité des réglages (tuiles vs preset)

Règle v1.0 pour éviter les conflits entre tuiles d’intensité, `smoothing` du preset et `LUMAWAY_COLOR_PROFILE` :

| Couche | Règle |
|--------|--------|
| **Mode** (`LUMAWAY_SYNC_MODE`) | Choisit preset CLI, `ColorProfile` par défaut, `capture_fps`, `smoothing` **de base** du preset (table §6). |
| **Tuiles Subtle→Max** | Au **Start** (et à chaque changement de tuile **hors sync**), écrivent `LUMAWAY_REACTIVITY` ; cette valeur devient le `--smoothing` effectif passé au subprocess — **elle écrase le `smoothing` du preset** pour la session. |
| **Curseur Luminosité** | `LUMAWAY_BRIGHTNESS` indépendant ; appliqué en plus du mode. |
| **`LUMAWAY_COLOR_PROFILE` en fichier** | Ignoré au Start si défini par le mode, sauf exception v1.0 : option **cinema** en mode Video. Les autres surcharges utilisateur de grading Hue sont post-v1.0 ; les profils capture restent non liés au mode. |
| **Changement de tuile pendant sync** | v1.0 : tuiles **désactivées** pendant sync (comme les modes) ; ajuster l’intensité = Stop → tuile → Start. |
| **Mode Jeu** | Tuile par défaut suggérée **High** ou **Max** au premier lancement (pas de tuiles désactivées). |
| **Mode Musique** | Même règle tuiles → `LUMAWAY_REACTIVITY` ; voir paragraphe « Mode Musique » au-dessus de ce tableau. |

### Correspondance contrôles Hue Sync → LumaWay

| Hue Sync (public) | LumaWay actuel / prévu |
|-------------------|------------------------|
| Luminosité | `LUMAWAY_BRIGHTNESS` / curseur GUI |
| Intensité / vitesse | Tuiles Doux→Max → `LUMAWAY_REACTIVITY` → `--smoothing` (poids frame courante) |
| Mode Video | `LUMAWAY_SYNC_MODE=video` + preset `video-wayland` |
| Mode Game | `LUMAWAY_SYNC_MODE=game` + preset `game-wayland` |
| Mode Audio Hue Sync / Musique LumaWay | `LUMAWAY_SYNC_MODE=music` + `lumaway audio-sync` |
| Bureau (implicite) | `LUMAWAY_SYNC_MODE=desktop` + preset `desktop-wayland` |
| Zone Entertainment | `LUMAWAY_AREA` + configuration dans l’app Hue |
| Sync Box / HDMI | Hors scope — matériel et pipeline distincts |

## 7. Structure code suggérée

```text
crates/lumaway-core/src/
  sync_mode.rs              # enum SyncMode + résolution preset
  audio/
    mod.rs
    source.rs               # trait AudioSource
    pipewire.rs
    pulse.rs
    analyzer.rs             # FFT, bandes, beat

crates/lumaway-cli/src/
  audio_sync_run.rs
  sync_run.rs               # refactor : boucle tick → couleurs → encode → send

crates/lumaway-gui/src/
  mode_selector.rs          # câblage mode_row + preset_row
  tray.rs
  onboarding.rs             # assistant première utilisation
  i18n.rs                   # init gettext, bindtextdomain, optional LUMAWAY_LANG
  user_messages.rs          # codes erreur CLI → gettext (UI)
po/
  LINGUAS, lumaway-gui.pot, fr.po, …
packaging/desktop/
  *.desktop avec nom/comment traduits (ou po4a)
  *.metainfo.xml avec nom/résumé traduits
```

**Refactor minimal** : extraire de `sync_run.rs` la boucle commune et brancher `ScreenColorProvider` vs `AudioColorProvider`.

## 8. Risques et mitigations

| Risque | Mitigation |
|--------|------------|
| Portal : frames noires en GL | CPU par défaut ; `backend-probe` / fallback auto. |
| Latence audio élevée | Buffers courts ; analyse 50 Hz ; pas de capture écran en parallèle. |
| Une zone Entertainment active | Documenter ; modes Écran et Musique mutuellement exclusifs. |
| Droits audio (Flatpak) | Valider d’abord en install natif ; permissions dans le manifest plus tard. |
| Algorithmes Signify inconnus | Tests sur motifs visuels/audio de référence, pas RE binaire. |
| Charge CPU | Mode Bureau : FPS capture bas ; Musique sans GStreamer écran. |
| UX trop technique | Phase 1.5 + checklist §3.4 avant nouvelles features moteur. |
| Tuiles mode factices | Priorité 1.1 — risque de frustration si l’UI « ment ». |
| Chaînes non extraites | Phase 0 gettext tôt ; revue « no hardcoded UI string » avant release. |
| `.mo` manquants après install | `install-desktop-app.sh` compile et installe les locales listées dans `LINGUAS`. |
| Jeu plein écran Wayland (Gamescope, etc.) | Doc : l’utilisateur doit choisir la **bonne fenêtre/écran** dans le sélecteur Portal ; pas de capture magique sans choix. |
| Firmware pont Hue v1 / v2 | Pas de branche spécifique prévue ; valider sur firmware courant ; problèmes → `doctor` + issue. |
| Subprocess CLI crash | GUI repasse en état Arrêté + message (tâche 1.13). |
| Tray GNOME non disponible | Ne pas dépendre du tray pour une fonction critique ; GTK documente que la zone de notification peut être absente ([GtkStatusIcon](https://gnome.pages.gitlab.gnome.org/gtk/gtk3/class.StatusIcon.html)), et GNOME expose le support tray via extensions [Status Icons](https://extensions.gnome.org/extension/7332/status-icons/) / [AppIndicator](https://extensions.gnome.org/extension/615/appindicator-support/). Fallback fenêtre Start/Stop obligatoire + notification minimale pour erreurs critiques si disponible. |

## 9. Ordre d’exécution

```text
Phase 0  Spéc modes + presets CLI + amorçage gettext (po/, en, fr)
    ↓
Phase 1  UX Hue Sync (câbler UI + i18n + épuré + tray + assistant)  ← priorité produit
    ↓
Phase 2  Qualité écran (crop, défauts)       ← partiellement en parallèle de Phase 1
    ↓
Phase 3  Spike audio-sync → GUI Musique
    ↓
Phase 4  Doctor, doc, Flatpak audio
```

Pour une **v1.0 écran quotidien** sans Phase 3, remonter avant release les éléments Phase 4 non audio requis par §15.2 : AppStream, README minimal, CI `msgfmt`, validation install script. Les éléments audio/Flatpak audio restent avec Phase 3 / v1.1.

**Estimation** : 12–16 semaines à temps partiel (Phase 1 = 16 tâches + robustesse P0) ; 8–10 semaines si Phase 0 + Phase 1 cœur (1.1–1.5, 1.11, 1.13–1.14) + spike Phase 3 sans assistant complet.

## 10. Prochaines actions

1. Valider wireframe §3.3, msgid stables et table réactivité §6 (Subtle→Max).
2. Phase 0 : `SyncMode` + presets CLI + `LUMAWAY_CONFIG_VERSION` + `po/` + gettext dans `lumaway-gui`.
3. Phase 1 en priorité : i18n, puis **1.1, 1.2, 1.3** (propagation env / preset), **1.4, 1.5, 1.11**, puis **1.13–1.14** (robustesse §15 P0, instance unique), tray/assistant.
4. Tests checklist §3.4, §3.5 et release §15.2 avec utilisateur non dev.
5. Spike Phase 3 : `audio-sync` puis activer tuile Musique.
6. Relire [`desktop-app.md`](desktop-app.md) après câblage GUI Phase 1 pour vérifier les modes visibles, gettext et le périmètre v1.0 (§15.6).
7. Ne plus modifier le périmètre sans entrée §15.7 + mise à jour **Dernière revue** (§16).

## 11. Suivi

| Phase | Statut | Notes |
|-------|--------|-------|
| 0 | Terminé | Contrat `SyncMode`, presets CLI, config v1, gettext, AppStream et install script vérifiés ; câblage UI complet et i18n exhaustive restent Phase 1 |
| 1 | En cours | Tâches 1.1 / 1.2 / 1.3 lancées ; 1.4 socle livré : tuiles Mode et Intensité branchées, Music désactivé, Start propage mode/réactivité/profil, accueil/réglages/statuts et erreurs principales traduits ; 1.5 livré : réglages techniques et journal repliés ; 1.6 partiel : guide première configuration + aucune zone Entertainment guidée ; 1.8 livré : autostart de session + autostart sync séparés ; 1.9 livré : actions `Retry` / `Open Settings` sur erreurs classifiées ; 1.10 livré : rappel Portal + persistance opportuniste `restore_token` ; 1.11 livré : bouton unique et Réglages bloqués pendant sync ; 1.13 P0 code livré : sortie subprocess inattendue, classification pont/Portal/DTLS, flux Portal fermé, pont perdu pendant envoi DTLS et reprise après veille détectés ; 1.14 livré : instance unique GUI ; 1.15 livré : modes bloqués pendant sync ; 1.16 livré : switch zone clarifié ; dialogue À propos livré |
| 2 | À faire | |
| 3 | À faire | |
| 4 | À faire | |

Mettre à jour ce tableau à chaque jalon livré.

## 12. Documentation et index

| Document | Rôle par rapport à ce plan |
|----------|----------------------------|
| [plan-hue-sync-daily.md](plan-hue-sync-daily.md) | Feuille de route produit (ce fichier) |
| [hue-sync-research.md](hue-sync-research.md) | Contraintes API Entertainment et parité technique écran |
| [capture-improvement-roadmap.md](capture-improvement-roadmap.md) | Détail Phase 2 (qualité capture/couleur) |
| [desktop-app.md](desktop-app.md) | Install GUI, `lumaway.env`, autostart |
| [security.md](security.md) | Pinning TLS, DTLS LAN, `LUMAWAY_BRIDGE_ID` |
| [backlog.md](backlog.md) | Historique jalons + éléments reportés |
| [test-matrix.md](test-matrix.md) | Matrice de tests à étendre pour chaque phase |
| [architecture-plan.md](architecture-plan.md) | Vision stack Rust / GTK / Flatpak |
| [open-questions.md](open-questions.md) | DTLS recovery, secrets, config |
| §3.5 (ce document) | gettext, `po/`, locales, périmètre traduit |
| §15 (ce document) | Lacunes, release v1.0, robustesse, décisions Scenes/intensité |
| §4 « Architecture actuelle » | GUI subprocess vs orchestrateur cible |
| §6.1 (ce document) | Priorité tuiles vs preset / `COLOR_PROFILE` |
| §15.6 (ce document) | Alignement `desktop-app.md`, `main.rs`, README |
| §15.6–§15.7 (ce document) | Revue doc + journal des corrections |
| §16 (ce document) | Gel du plan et règles de modification |
| Début du document | [Comment lire ce plan](#comment-lire-ce-plan), [Table des matières](#table-des-matières) |

## 13. Éléments reportés (hors phases immédiates)

Liés au produit mais **non bloquants** pour la v1.0 « quotidien écran » — suivis dans [backlog.md](backlog.md) :

| Élément | Statut | Lien plan |
|---------|--------|-----------|
| Manifest Flatpak complet | Reporté | Phase 4 (permissions Portal + audio) |
| Secret Service (`libsecret` / `oo7`) | Reporté | [open-questions.md](open-questions.md) — remplacer clés en clair dans `lumaway.env` |
| Migration depuis Lumux | Reporté | Import config pont/zone si format documenté |
| `lumaway pin-status` / alerte changement IP pont | Sécurité | [security.md](security.md), pas ce plan produit |
| Placement 3D complet | Post-v1.0 | Phase 2.6 |
| Mapping spatial audio multi-canal | v2 | Musique — extension v2 (§5) |

## 14. Checklist de complétude du **document** plan

Indique si le **plan écrit** couvre le sujet — **pas** l’état d’implémentation du code.

| Sujet couvert par ce document | Plan |
|-------------------------------|------|
| Parité UX / user-friendly (§3) | Oui |
| i18n (§3.5) | Oui |
| Quatre modes + décision Scenes (§6) | Oui |
| Phases 0–4 + musique | Oui |
| Lacunes & release v1.0 (§15) | Oui |
| Alignement `test-matrix` / `architecture-plan` | Oui (§15) |
| Architecture subprocess vs cible (§4) | Oui |
| `LUMAWAY_PRESET` / propagation env (§6) | Oui |
| Table intensité + mode Musique (§6) | Oui |
| Priorité tuiles vs preset (§6.1) | Oui |
| Incohérences §3.4 / tâche 1.15 harmonisées | Oui |
| Latence 300 ms → Phase 2.4 / release | Oui |
| Mode Musique v1.1 / `cinema` avancé v1.0 / veille v1.0 | Oui (§6, §1, §15.3) |
| Gel et règles de modification (§16) | Oui |

**Implémentation produit** : suivre le tableau §11 (Phases 0–4) et la checklist utilisateur §3.4 (cases à cocher lors des tests manuels).

## 15. Lacunes identifiées et critères release

Complète les phases ci-dessus ; priorités pour ne pas livrer une v1.0 « cassante » au quotidien.

### 15.1 Décisions produit (à respecter en implémentation)

| Sujet | Décision |
|-------|----------|
| Tuile **Scenes** | Supprimée / remplacée par **Desktop** (§6). |
| **Luminosité + Intensité** | Curseur luminosité + tuiles d’intensité uniquement ; pas de double curseur. |
| **Changement de mode pendant sync** | v1.0 : **Stop** puis nouveau mode ; pas de bascule à chaud. |
| **Données** | Traitement **100 % local** ; **pas de télémétrie** — mentionner dans Réglages / dialogue **À propos**. |
| **Environnements v1.0** | **GNOME Wayland** = référence release ; KDE / Sway / Hyprland = tests best-effort ([test-matrix.md](test-matrix.md)). |
| **Mode Musique + intensité** | Tuiles Subtle→Max → `LUMAWAY_REACTIVITY` ; styles Ambiant/Fête via `LUMAWAY_MUSIC_STYLE` (§6). |
| **`LUMAWAY_PRESET`** | Déprécié au profit de `LUMAWAY_SYNC_MODE` ; alias `tv-wayland` conservé. |
| **Priorité tuiles / preset** | §6.1 : tuiles → `LUMAWAY_REACTIVITY` écrase `smoothing` preset au Start. |
| **Vidéo `ColorProfile`** | Défaut **`vivid`** ; `cinema` avancé uniquement. |
| **Dialogue À propos** | Requis pour la promesse « données locales / pas de télémétrie ». |
| **Veille / reprise** | v1.0 : Stop + message ; pas de reconnexion automatique silencieuse. |
| **`cinema` avancé** | Option Réglages ; prioritaire sur `vivid` en mode Video au Start (§6). |
| **Musique / versioning** | v1.0 ne bloque pas sur Musique ; Phase 3 devient v1.1 si elle n’est pas terminée avant le gel v1.0. |
| **Tray GNOME** | Le tray est opportuniste : requis quand StatusNotifier/AppIndicator est disponible, mais la v1.0 doit rester utilisable sans extension tray via fenêtre Start/Stop, avec notification minimale si disponible. |
| **Gel du document** | Modifications via §15.7 + §16. |

### 15.2 Critères release v1.0

Tous requis sauf mention « optionnel » :

- [ ] Checklist §3.4 validée sur GNOME Wayland (utilisateur non dev).
- [ ] `en` + `fr` : écran principal 100 % traduit ; repli `en` sans msgid brut.
- [ ] Modes Video / Game / Desktop branchés ; Music branché si Phase 3 livrée (sinon tuile désactivée + doc).
- [ ] Tray Start/Stop opérationnel quand le bureau expose StatusNotifier/AppIndicator ; sur GNOME vanilla, fenêtre Start/Stop validée et notification critique testée si disponible.
- [ ] Robustesse §15.3 P0 passée sur matrice test (sleep, pont perdu, Portal fermé, **subprocess terminé avec erreur**).
- [ ] Changement de mode **uniquement après Stop** (tâche 1.15) validé manuellement.
- [ ] Harness / test latence : réaction perceptible **&lt; 300 ms** sur plein écran (Phase 2.4, §5).
- [ ] `install-desktop-app.sh` installe binaires + `.mo` + `.desktop` + AppStream `.metainfo.xml`.
- [ ] Aucune clé API sur l’écran principal ; pairing guidé.
- [x] Dialogue **À propos** (version, confidentialité locale, pas de télémétrie).
- [ ] Flatpak : optionnel v1.0 (peut suivre en v1.1 si install script suffit).
- [ ] KDE Wayland : smoke test best-effort (non bloquant si GNOME OK).

### 15.3 Backlog lacunes par priorité

<a id="p0--avant-ou-avec-la-v10-quotidien"></a>
#### P0 — avant ou avec la v1.0 « quotidien écran »

| Lacune | Livrable | Phase |
|--------|----------|-------|
| Reprise après **veille** / session Portal expirée | Livré partiel : écart horloge murale / monotone > 5 s → arrêt propre, désactivation Entertainment tentée, message i18n “sortie de veille” + `Retry` ; validation manuelle encore requise | 1.13 |
| **Pont injoignable** ou perdu pendant sync | Livré partiel : échec d’envoi DTLS pendant sync → arrêt propre, désactivation Entertainment tentée, message i18n “connexion pont perdue” + `Retry` / `Open Settings` ; validation manuelle encore requise | 1.13 |
| **Flux Portal fermé** pendant sync | Livré partiel : absence de frames > 5 s → arrêt propre + désactivation Entertainment + message i18n ; validation manuelle encore requise | 1.13 |
| **Échec DTLS** / handshake | **3 tentatives au total** puis message i18n ; doc comportement complet en Phase 4 ([open-questions.md](open-questions.md)) | 1.13 + 4 |
| **Fallback GNOME sans tray** | Fenêtre Start/Stop toujours accessible ; notification minimale pour erreurs critiques si disponible | 1.7 / 1.13 |
| **Conflit Entertainment** (autre app / zone) | Message : une seule zone active ; actions `Retry` / `Open Settings` selon le contexte | 1.9 |
| **Aucune zone** configurée | Livré partiel : état accueil “No Entertainment zone” + journal guidant la création d’une zone Entertainment dans l’app Hue, ajout de lumières, puis rechargement via Réglages > Save ; assistant complet reste 1.6 | 1.6 |
| **Instance unique** GUI | Une seule fenêtre / une seule sync ; second lancement active l’instance existante | 1.14 |
| **À propos** | Livré : `AdwAboutDialog` depuis l’accueil avec version Cargo, licence MPL-2.0, dépôt/issues GitHub, données locales, pas de télémétrie, mention **Philips Hue** nominative + non-affiliation Signify | 1 |
| **AppStream** | `io.github.BunnySweety.LumaWay.metainfo.xml` traduit en + fr, installé par le script, validable par `appstreamcli` si présent | 0 / 4 |
| **Migration** `lumaway.env` | `LUMAWAY_CONFIG_VERSION` ; alias `LUMAWAY_PRESET` → `LUMAWAY_SYNC_MODE` | 0 |
| **Subprocess `lumaway` terminé** (code ≠ 0, signal) | GUI : état Arrêté + message i18n, pas de spinner infini | 1.13 |

#### P1 — forte valeur UX, peut suivre juste après v1.0

| Lacune | Livrable | Phase |
|--------|----------|-------|
| **Notifications bureau enrichies** (actions détaillées, erreurs pont/Portal persistantes) | `libnotify` ou portail GNOME au-delà du fallback minimal P0 | 1.13+ |
| **Changement IP pont** avec `LUMAWAY_BRIDGE_ID` | Alerte + `discover-bridges` / pin ([security.md](security.md)) | 4 / sécurité |
| **Limite ~10 lampes** / zone inadaptée | Avertissement si une lampe ou zone « qualité » faible | 2 |
| **CI `msgfmt --check`** | Workflow GitHub | 0 / 4 |
| **Réinitialiser la configuration** | Effacer / réinitialiser `lumaway.env` (+ profils optionnel) avec confirmation | 1 |
| **Mémorisation Portal** | `restore_token` sauvegardé dans `LUMAWAY_PORTAL_RESTORE_TOKEN` quand exposé par le portail ; sinon rappel à chaque session | 1.10 |
| **Changement mode à chaud** | Sans Stop | post-v1.0 |
| **Secret Service** pour clés | [open-questions.md](open-questions.md) | reporté §13 |

#### P2 — post-v1.0

| Lacune | Notes |
|--------|--------|
| HDR / espace colorimétrique capture | [capture-improvement-roadmap.md](capture-improvement-roadmap.md) |
| Profil **batterie** (réduire `capture_fps`) | Portable Linux |
| **Weblate** / guide contributeurs `po/` | i18n communautaire |
| **Accessibilité** (a11y GTK) | Labels, focus, lecteur d’écran |
| Packaging **AUR** / Flathub stable | Au-delà du script install |
| **lumaway-config** (TOML utilisateur) | README « Planned » |

### 15.4 Alignement documents

| Document | Écarts comblés par §15 |
|----------|-------------------------|
| [architecture-plan.md](architecture-plan.md) | Arrêt/reprise bridge, multi-écran, bridge perdu |
| [test-matrix.md](test-matrix.md) | Scénarios sleep, hotplug, DTLS, i18n, robustesse |
| [security.md](security.md) | `pin-status`, IP vs `BRIDGE_ID` → P1 |
| [backlog.md](backlog.md) | Multi-moniteur → Phase **1.10** (plus 1.7) |

### 15.5 Risques additionnels

| Risque | Mitigation |
|--------|------------|
| Hyperion / Hue app monopolise Entertainment | Message conflit §15.3 P0 |
| Checklist §14 confondue avec « fait » | §14 renommé « document plan » |
| Sous-sections « 3.x » dans Phase Musique | Renommées « Musique — … » |
| GUI subprocess vs orchestrateur | Documenté §4 ; refactor in-process post-v1.0 |
| Tuiles vs curseur Hue Sync | Choix produit documenté §3.1 |
| `cinema` / `vivid` | Défaut `vivid` §6 |
| Tray GNOME | Fallback fenêtre Start/Stop documenté ; notification critique seulement si disponible ; tray testé seulement quand StatusNotifier/AppIndicator est disponible |
| AppStream demandé sans tâche | Métadonnées ajoutées en Phase 0 et release §15.2 |
| Musique v1.0 ambiguë | v1.0 = écran quotidien ; Musique = v1.1 si Phase 3 non terminée avant gel |

### 15.6 Revue documentaire (alignement à l’implémentation)

| Fichier | Action |
|---------|--------|
| [`desktop-app.md`](desktop-app.md) | Aligné Phase 0 sur `LUMAWAY_SYNC_MODE`, Video/Game/Desktop et profils techniques avancés ; à relire après finalisation UX Phase 1 |
| [`lumaway-gui`](../crates/lumaway-gui/src/main.rs) | Phase 1 lancée : tuiles Video/Game/Desktop branchées, Music grisé, tuiles Subtle→Max branchées, preset/profil couleur dérivés au Start, accueil/réglages/statuts et erreurs principales via gettext ; Phase 1.5 replie clés, profils, durée, réactivité fine, Quality/Calibrate et journal dans Réglages ; Phase 1.9 affiche des actions de récupération sur les erreurs classifiées ; Phase 1.13 classe flux Portal fermé, pont perdu pendant sync et reprise après veille |
| README | Section « Comparaison Hue Sync » + guide traduction (Phase 4) |
| [`test-matrix.md`](test-matrix.md) | Garder aligné avec §15.2 et §15.3 à chaque jalon |

### 15.7 Journal des corrections (traçabilité)

| Date / passe | Sujet | Résolution |
|--------------|-------|------------|
| 2026-05-16 | Phase 1.6 guide | Carte “First setup” sur l’accueil : Discover, Pair, Test lights (`test-color red`), progression jusqu’à Start sync |
| 2026-05-16 | Phase 1.6 aucune zone | Retour `list-areas` vide transformé en état guidé : zone désactivée, en-tête clair, étapes app Hue puis rechargement |
| 2026-05-16 | Phase 1 À propos | Dialogue `AdwAboutDialog` ajouté : version, MPL-2.0, dépôt, confidentialité locale, zéro télémétrie, mention Philips Hue nominative |
| 2026-05-16 | Phase 1.13 veille | Reprise après veille détectée par écart horloge murale / monotone > 5 s ; sync arrêtée avec message i18n et action `Retry` |
| 2026-05-16 | Phase 1.13 pont perdu | Échec d’envoi DTLS pendant sync annoté “bridge lost during sync” et mappé GUI vers message i18n + actions `Retry` / `Open Settings` |
| 2026-05-16 | Phase 1.13 Portal fermé | Timeout de flux Portal sans nouvelle frame > 5 s converti en erreur classifiée ; la sortie subprocess déclenche l’état arrêté et `Retry` |
| 2026-05-16 | Phase 1.10 Portal | Statut traduit pendant le sélecteur Portal ; `restore_token` persistant via `LUMAWAY_PORTAL_RESTORE_TOKEN` quand disponible |
| 2026-05-16 | Phase 1.9 erreurs | Actions contextuelles `Retry` et/ou `Open Settings` affichées sous Start pour les erreurs GUI classifiées |
| 2026-05-16 | Phase 1.8 autostart | Option Réglages pour créer/supprimer l’entrée XDG autostart ; option sync à l’ouverture gardée séparée |
| 2026-05-15 | Phase 1.16 switch zone | Tooltip traduit : le switch contrôle l’allumage zone Hue, Start/Stop contrôle la sync écran |
| 2026-05-15 | Phase 1.15 modes | Tuiles Mode désactivées pendant sync ; tooltip traduit indiquant Stop avant changement de mode |
| 2026-05-15 | Phase 1.14 instance unique | Activation secondaire GTK présente la fenêtre existante ; pas de seconde fenêtre ni seconde sync créée |
| 2026-05-15 | Phase 1.13 subprocess | Sortie inattendue de `lumaway sync` distinguée des arrêts/restarts demandés ; dernier log d’erreur mappé via `user_messages` pour état pont/Portal/capture/DTLS |
| 2026-05-15 | Phase 1.11 bouton sync | Libellé dynamique Start sync / Stop sync ; bouton/fenêtre Réglages et contrôles de configuration désactivés pendant sync |
| 2026-05-15 | Phase 1.5 écran épuré | Accueil sans champs techniques ; Réglages garde pont/zone/luminosité/autostart visibles, clés/profils/durée/réactivité fine/Quality/Calibrate et journal repliés |
| 2026-05-15 | Phase 1.4 `user_messages` | Codes stables GUI ajoutés pour Hue auth, pont injoignable, DTLS, conflit Entertainment, Portal et capture ; rendu gettext |
| 2026-05-15 | Phase 1.4 i18n GUI | Accueil, Réglages et statuts principaux migrés vers gettext ; `po/fr.po` étendu |
| 2026-05-15 | Phase 1.2 intensité | `preset_row()` branché sur `LUMAWAY_REACTIVITY` ; défaut Jeu = High sans valeur sauvegardée ; tuiles désactivées pendant sync |
| 2026-05-15 | §3.4 vs tâche 1.15 | Test réactivité = après Stop + nouvelle sync |
| 2026-05-15 | Tuiles vs `smoothing` preset | §6.1 : tuiles écrasent preset au Start |
| 2026-05-15 | Vidéo `cinema` / `vivid` | Défaut `vivid` ; `cinema` via Réglages avancé (§6) |
| 2026-05-15 | §13 « Phase 3.4 » | → Musique — extension v2 |
| 2026-05-15 | Latence &lt; 300 ms | Phase 2.4 + §15.2 (plus de renvoi §2.4 erroné) |
| 2026-05-15 | DTLS P0 vs Phase 4 / code actuel | v1.0 = 3 tentatives de handshake initial ; doc complète Phase 4 |
| 2026-05-15 | gettext / i18n-embed | gettext + install script en v1.0 |
| 2026-05-15 | `COLOR_PROFILE` vs mode | Mode gagne ; exception `cinema` avancé en mode Video |
| 2026-05-15 | À propos | Requis v1.0 et classé P0 |
| 2026-05-15 | Jeu `capture_fps` | Valeur unique **12** en §6 |
| 2026-05-15 | Musique + réactivité | `LUMAWAY_REACTIVITY` → poids FFT dans `audio-sync` (§6) |
| 2026-05-15 | Veille / reprise | v1.0 = Stop + message ; auto-reconnect post-v1.0 (§1, §15.3) |
| 2026-05-15 | Ordre §15.6 / §15.7 | Revue doc avant journal |
| 2026-05-15 | Tray GNOME | Tray opportuniste ; fallback fenêtre Start/Stop obligatoire + notification minimale si disponible sur GNOME sans extension |
| 2026-05-15 | Références internes | Remplacer les renvois de section ambigus par tâches 1.15/1.16 |
| 2026-05-15 | AppStream | Métadonnées `.metainfo.xml` ajoutées aux livrables Phase 0 / release |
| 2026-05-15 | Musique v1.0 | v1.0 = écran quotidien ; Musique = v1.1 si Phase 3 non terminée avant gel |
| 2026-05-15 | Notifications fallback tray | Notification minimale P0 ; notifications enrichies restent P1 |
| 2026-05-15 | Crate i18n | Dépendance figée sur `gettext-rs` / crate `gettextrs` |
| 2026-05-15 | `desktop-app.md` | Aligné Phase 0 sur `LUMAWAY_SYNC_MODE`, modes et profils techniques avancés ; relecture finale après Phase 1 |
| 2026-05-15 | Réglages source audio | Source audio visible seulement quand Musique / Phase 3 est livrée |
| 2026-05-15 | Critère Start/Stop | Fenêtre Start/Stop obligatoire ; notification ne remplace pas l’action principale |
| 2026-05-15 | Promesse zéro terminal | Bornée à la configuration et l’usage après installation, car v1.0 peut rester sur install script |
| 2026-05-15 | Nomenclature Audio/Musique | Correspondance explicitée : Audio Hue Sync = Musique LumaWay ; tooltip Musique traduit |

## 16. Gel du plan et modifications

- Toute **modification de périmètre** après cette date passe par une ligne dans **§15.7** et une mise à jour de **Dernière revue** en tête de fichier. La [table des matières](#table-des-matières) et le guide [Comment lire ce plan](#comment-lire-ce-plan) restent à jour si des sections sont ajoutées ou renommées.
- Les **valeurs numériques** marquées « bench Phase 0 » (`capture_fps` Jeu, tuiles Subtle→Max) ne changent qu’après mesure documentée dans le harness Phase 2.4 ou un commit dédié.
- **Source de vérité implémentation** : §6 + §6.1 pour les modes ; §5 Phase 0–4 pour l’ordre de livraison ; §15.2 pour la definition of done v1.0.
