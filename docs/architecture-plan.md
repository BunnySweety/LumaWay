# Projet : LumaWay

## Objectif

Créer une nouvelle application Hue Sync pour Linux/Wayland, nommée **LumaWay**, pensée comme une vraie alternative à Lumux plutôt qu'un fork incrémental.

L'objectif principal est d'avoir une base durable, performante et maintenable pour :

- capturer l'écran sous Wayland ;
- analyser les couleurs en temps réel ;
- synchroniser les lumières Philips Hue via Hue Entertainment ;
- proposer une interface native Linux propre ;
- distribuer l'application proprement via Flatpak.

Le périmètre volontaire du projet est **Linux Wayland**. Les choix techniques ne doivent pas être dilués par des objectifs Windows, macOS ou X11.

**Plan produit** : [plan-hue-sync-daily.md](plan-hue-sync-daily.md).


## Décision technique recommandée

### Stack principale

**Rust + GTK4/libadwaita + Flatpak**

C'est le choix le plus cohérent pour ce type d'application.

### Décision retenue

```text
Nom : LumaWay
Licence : MPL-2.0
Plateforme : Linux Wayland
Langage : Rust
UI : GTK4/libadwaita
Distribution : Flatpak
Architecture : moteur headless d'abord, UI ensuite
Premier livrable : CLI Hue DTLS couleur fixe
Binaire CLI : lumaway
```

App ID Flatpak provisoire :

```text
io.github.BunnySweety.LumaWay
```

Le nom `LumaWay` est retenu pour donner au projet une identité distincte de Lumux tout en conservant un signal clair autour de la lumière et de Wayland.

## Pourquoi Rust

Rust est adapté au coeur temps réel de l'application :

- meilleure maîtrise de la latence ;
- meilleure sécurité mémoire ;
- meilleur contrôle sur les threads, buffers et flux vidéo ;
- bon écosystème pour Linux natif ;
- distribution plus robuste qu'une application Python ;
- plus adapté au réseau bas niveau, UDP et DTLS.

## Pourquoi GTK4/libadwaita

GTK4/libadwaita est le meilleur choix pour une application Linux/Wayland native :

- excellente intégration GNOME/Linux ;
- cohérent avec Flatpak ;
- accès naturel à GLib, Gio, D-Bus, portals ;
- interface moderne et native ;
- pas de webview inutile pour une application bas niveau.

## Pourquoi Flatpak

Flatpak colle bien au modèle de sécurité nécessaire :

- permissions explicites ;
- intégration avec XDG Desktop Portal ;
- distribution Linux simple ;
- packaging reproductible ;
- bon support pour les apps GTK/libadwaita.

## Pourquoi éviter Tauri en premier choix

Tauri est intéressant pour des apps desktop avec UI web moderne, mais ici le coeur du produit est bas niveau :

- capture écran Wayland ;
- PipeWire ;
- GStreamer ;
- D-Bus ;
- UDP/DTLS ;
- permissions Flatpak ;
- faible latence.

Une webview n'apporte pas grand-chose à ces besoins et ajoute une couche supplémentaire.

## Pourquoi éviter Go

Go est bon pour réseau et concurrence, mais moins adapté ici :

- intégration desktop Linux moins naturelle ;
- bindings GTK/GStreamer moins confortables ;
- risque de dépendance forte à CGO ;
- moins bon choix pour une app GNOME/Wayland/Flatpak native.

## Pourquoi ne pas garder Python pour une vraie alternative

Python peut fonctionner, et Lumux prouve que c'est possible.

Mais pour une vraie alternative ambitieuse :

- la distribution est plus fragile ;
- le coeur temps réel est moins maîtrisé ;
- les bindings bas niveau ajoutent des couches ;
- le DTLS Hue risque de rester bricolé ;
- les performances et la stabilité seront plus difficiles à garantir.

Python reste valable pour prototyper, mais pas idéal pour repartir proprement.

# Validation technique avant MVP

Avant de construire l'application complète, il faut traiter le projet comme une validation de risques. Les trois risques majeurs doivent être prouvés séparément :

1. **Hue Entertainment DTLS natif en Rust**
   - établir une connexion DTLS sans subprocess `openssl` ;
   - envoyer une frame HueStream valide ;
   - activer et désactiver proprement une Entertainment Area.

2. **Capture Wayland fiable**
   - demander l'autorisation via XDG Desktop Portal ;
   - récupérer les frames via PipeWire/GStreamer ;
   - fonctionner dans un environnement Flatpak ;
   - supporter les cas de refus, fermeture de session et reprise après veille.

3. **Latence stable**
   - maintenir une sync continue sur plusieurs minutes ;
   - mesurer chaque étape du pipeline ;
   - éviter les files d'attente qui accumulent du retard ;
   - dégrader proprement en cas de surcharge CPU.

Tant que ces trois points ne sont pas validés, l'interface graphique doit rester secondaire.

# Objectifs mesurables

Les décisions techniques doivent être guidées par des métriques concrètes :

- 30 FPS cible pour le MVP ;
- 15 FPS minimum acceptable sur machine modeste ;
- latence totale cible sous 50 ms après réception d'une frame ;
- session de sync stable pendant au moins 30 minutes sans crash ;
- arrêt/reprise propre après perte du bridge ;
- arrêt/reprise propre après fermeture ou révocation du flux portal ;
- pas d'accumulation de frames en retard ;
- logs exploitables pour chaque erreur utilisateur probable.

# Périmètre plateforme

## Plateforme cible

La plateforme cible est :

```text
Linux + Wayland + XDG Desktop Portal + PipeWire + Flatpak
```

Les environnements prioritaires sont :

- GNOME Wayland ;
- KDE Wayland ;
- wlroots/Sway ;
- Hyprland ;
- Flatpak sandbox.

## Hors périmètre

Ces environnements ne doivent pas influencer l'architecture initiale :

- Windows ;
- macOS ;
- X11 natif ;
- applications mobiles ;
- navigateur web ;
- daemon cloud.

Le projet peut rester techniquement portable quand cela ne coûte rien, mais aucune abstraction ne doit être ajoutée uniquement pour un support hors Linux Wayland.

# Architecture cible

```text
app/
  docs/
    adr/
    benchmarks/
    test-matrix.md

  crates/
    lumaway-core/
      capture/
      colors/
      zones/
      sync/
      metrics/
      diagnostics/

    lumaway-hue/
      rest/
      entertainment/
      discovery/

    lumaway-cli/

    lumaway-gtk/
      windows/
      dialogs/
      widgets/
      preview/

    lumaway-config/
      settings/
      profiles/
      secrets/
```

## Crate `core`

Responsabilités :

- capture écran ;
- gestion du pipeline vidéo ;
- downscale ;
- extraction des zones ;
- analyse des couleurs ;
- smoothing ;
- brightness/gamma ;
- détection bandes noires plus tard ;
- boucle de synchronisation ;
- métriques FPS/latence ;
- diagnostics runtime.

## Crate `hue`

Responsabilités :

- découverte du Hue Bridge ;
- authentification ;
- client REST Hue v2 ;
- récupération des lumières ;
- récupération des Entertainment Areas ;
- activation/désactivation du mode Entertainment ;
- protocole HueStream ;
- streaming DTLS.

## Crate `gtk-app`

Responsabilités :

- interface GTK4/libadwaita ;
- assistant de configuration ;
- sélection du bridge ;
- sélection de l'Entertainment Area ;
- preview des zones ;
- start/stop sync ;
- diagnostics ;
- préférences utilisateur.

## Crate `config`

Responsabilités :

- settings utilisateur ;
- profils ;
- valeurs par défaut ;
- migration de config ;
- stockage des secrets ;
- import/export.

## Mode headless

Le moteur doit être utilisable sans interface graphique. C'est indispensable pour tester, diagnostiquer et développer sans dépendre de GTK.

Commandes cibles :

```text
lumaway discover-bridges
lumaway auth --bridge <ip>
lumaway list-areas --bridge <ip>
lumaway test-color --bridge <ip> --area <id> --color red
lumaway capture-stats
lumaway sync --bridge <ip> --area <id>
lumaway doctor
```

Le binaire GTK peut utiliser le même moteur que ces commandes, mais l'UI ne doit pas être nécessaire pour valider le pipeline.

# Décisions d'architecture

Le projet doit conserver des ADR courts dans `docs/adr/`.

ADR à écrire dès le début :

- choix de Rust ;
- choix de GTK4/libadwaita ;
- choix de GStreamer avant PipeWire direct ;
- choix de Flatpak comme canal principal ;
- choix du stockage secrets ;
- choix de l'implémentation DTLS.

Chaque ADR doit expliquer :

- le contexte ;
- la décision ;
- les alternatives considérées ;
- les conséquences ;
- les critères qui justifieraient de revenir dessus.

# MVP recommandé

## Fonctionnalités du MVP

Le MVP doit être très ciblé :

1. Connexion Hue Bridge.
2. Authentification avec bouton physique du bridge.
3. Sélection d'une Entertainment Area.
4. Capture écran via XDG Desktop Portal + PipeWire.
5. Extraction de couleurs sur 4, 8 ou 16 zones.
6. Streaming Hue Entertainment via DTLS.
7. Interface simple avec :
   - état du bridge ;
   - état de la capture ;
   - état du streaming ;
   - FPS ;
   - latence ;
   - erreurs exploitables ;
   - bouton Start/Stop ;
   - brightness ;
   - smoothing.
8. Commande `doctor` pour diagnostiquer l'environnement.
9. Build Flatpak fonctionnel.

## Fonctionnalités à repousser après le MVP

À ne pas faire au début :

- tray icon ;
- autostart ;
- profils avancés ;
- black bar detection ;
- multi-monitor avancé ;
- mode lecture ;
- thèmes complexes ;
- animations UI ;
- presets avancés ;
- AppImage ;
- support non-Linux ;
- effets musicaux ;
- marketplace de plugins ;
- synchronisation cloud.

## Non-objectifs initiaux

Ces points ne doivent pas guider l'architecture du MVP :

- pas de support Windows/macOS ;
- pas de support Hue Bridge v1 si cela complique le MVP ;
- pas de web UI embarquée ;
- pas de moteur d'effets avancé ;
- pas de thème custom complexe ;
- pas de système de plugins ;
- pas d'intégration cloud ;
- pas de compatibilité totale avec toutes les distributions dès le premier livrable.

# Matrice de test Linux Wayland

La matrice de test doit rester centrée sur Linux Wayland.

## Environnements prioritaires

```text
GNOME Wayland
KDE Wayland
Sway / wlroots
Hyprland
Flatpak sandbox
```

## Scénarios à tester

- premier lancement ;
- autorisation portal acceptée ;
- autorisation portal refusée ;
- reprise après fermeture du flux de capture ;
- reprise après veille ;
- hotplug écran ;
- multi-écran ;
- bridge Hue indisponible ;
- bridge Hue perdu pendant la sync ;
- Entertainment Area absente ;
- DTLS handshake échoué ;
- plugins GStreamer manquants ;
- exécution dans Flatpak ;
- exécution hors Flatpak pour développement.

## Critères de validation

- l'application explique l'erreur ;
- aucun secret n'apparaît dans les logs ;
- le moteur s'arrête proprement ;
- l'UI reste responsive ;
- `lumaway doctor` identifie la cause probable.

# Stack technique détaillée

## Langage

```text
Rust stable
```

## UI

```text
gtk4
libadwaita
glib
gio
```

## Portals / D-Bus

Options :

```text
ashpd
zbus
```

Priorité : utiliser `ashpd` si l'abstraction couvre correctement le besoin, sinon `zbus`.

## Capture vidéo

Options :

```text
GStreamer Rust bindings
PipeWire direct
```

Recommandation initiale :

```text
GStreamer Rust bindings
```

PipeWire direct peut venir plus tard si GStreamer limite trop le contrôle ou les performances.

## Hue REST API

Options :

```text
reqwest
ureq
```

Recommandation :

```text
reqwest
```

## Sérialisation

```text
serde
serde_json
toml
```

## Secrets

Options :

```text
Secret Service
libsecret
oo7
```

But :

- ne pas stocker `app_key` et `client_key` en JSON brut ;
- garder seulement les réglages non sensibles dans la config.

## Logging

```text
tracing
tracing-subscriber
```

Les logs doivent être structurés et pouvoir alimenter :

- une sortie CLI lisible ;
- un export diagnostic ;
- une vue diagnostics dans l'application GTK.

## Tests

```text
cargo test
```

À tester tôt :

- conversion RGB/XY ;
- extraction de zones ;
- smoothing ;
- mapping zones vers channels Hue ;
- construction des frames HueStream ;
- parsing des réponses Hue REST ;
- migrations de config.

## Diagnostics

Prévoir un module `diagnostics` dès le début, exposé par `lumaway doctor` et par l'UI.

Il doit vérifier :

- présence de `xdg-desktop-portal` ;
- backend portal actif ;
- disponibilité de l'interface ScreenCast ;
- disponibilité de PipeWire ;
- présence des plugins GStreamer nécessaires ;
- permission réseau dans Flatpak ;
- bridge Hue joignable ;
- credentials Hue valides ;
- Entertainment Areas détectées ;
- activation Entertainment possible ;
- connexion DTLS possible ;
- version de l'app, runtime Flatpak, environnement desktop.

Le résultat doit être structuré :

```text
check_id
status: ok | warning | error
message
hint
raw_detail
```

## Permissions Flatpak

Le packaging doit documenter précisément les permissions nécessaires :

- accès Wayland ;
- accès DRI si nécessaire au pipeline vidéo ;
- accès réseau local pour le bridge Hue ;
- accès aux portals via D-Bus ;
- accès PipeWire via XDG Desktop Portal ;
- stockage config XDG ;
- stockage secrets via Secret Service.

Le plan doit également documenter les limitations connues selon les environnements :

- GNOME ;
- KDE ;
- wlroots/Sway ;
- Hyprland ;
- sessions avec plusieurs écrans ;
- reprise après veille ;
- hotplug écran.

# Benchmark harness

Le projet doit intégrer un mode benchmark reproductible.

Commandes cibles :

```text
lumaway bench-capture --duration 60
lumaway bench-sync --duration 60 --area <id>
lumaway bench-colors --input sample-frame.png
```

Métriques à collecter :

- FPS moyen ;
- FPS p95/p99 ;
- latence capture ;
- latence analyse couleurs ;
- latence mapping ;
- latence envoi Hue ;
- latence totale ;
- CPU moyen ;
- mémoire ;
- frames ignorées ;
- erreurs par minute ;
- reconnexions.

Sorties :

```text
human-readable table
json report
optional CSV
```

Les benchmarks doivent servir à comparer les changements de pipeline, pas à optimiser prématurément.

# Threat model léger

Le projet doit avoir un threat model court, centré sur l'application locale Linux Wayland.

Actifs à protéger :

- `app_key` Hue ;
- `client_key` Hue ;
- rapports diagnostics ;
- logs ;
- accès au flux écran via portal.

Risques principaux :

- fuite de secrets dans logs ou rapports ;
- permissions Flatpak trop larges ;
- surface réseau locale non bornée ;
- erreurs de validation dans le client Hue REST ;
- comportement imprévisible si le bridge renvoie une réponse inattendue.

Mesures attendues :

- stockage secrets hors config JSON ;
- logs expurgés ;
- permissions Flatpak minimales ;
- validation stricte des entrées réseau ;
- timeout et retry bornés ;
- suppression des secrets depuis l'UI.

# Identité projet

Pour une vraie alternative, le projet doit avoir une identité distincte.

Décisions retenues :

- nom : `LumaWay` ;
- licence : `MPL-2.0` ;
- app id Flatpak provisoire : `io.github.BunnySweety.LumaWay` ;
- binaire CLI : `lumaway` ;
- crates initiales : `lumaway-hue`, `lumaway-cli`, `lumaway-core`, `lumaway-gtk`, `lumaway-config` ;
- plateforme annoncée : Linux Wayland ;
- positionnement : Hue Sync natif Linux Wayland.

À décider ensuite :

- icône minimale ;
- description courte ;
- politique de compatibilité.

Contraintes :

- éviter un nom trop proche de Lumux ;
- éviter une identité qui suggère un support Windows/macOS ;
- choisir un app id cohérent avec une distribution Flatpak future.

# Compatibilité Hue cible

Le MVP doit cibler explicitement :

- Hue Bridge v2 ;
- Hue API v2 ;
- Entertainment Areas ;
- lumières couleur compatibles Entertainment ;
- gradient lights si elles sont exposées via les channels Entertainment.

À documenter :

- comportement si une lumière ne supporte pas Entertainment ;
- comportement si une zone contient moins de lumières que prévu ;
- comportement si le bridge n'expose pas de `client_key` ;
- comportement si plusieurs Entertainment Areas existent ;
- limites connues des modèles Hue non testés.

# Point critique : Hue Entertainment DTLS

Le plus gros risque technique est le streaming Hue Entertainment.

Avant de construire l'UI, il faut prouver que Rust peut :

1. authentifier l'application auprès du bridge ;
2. récupérer le `client_key` ;
3. récupérer l'Entertainment Area ;
4. activer le streaming ;
5. ouvrir une connexion DTLS ;
6. envoyer une frame HueStream valide ;
7. changer une lumière de couleur avec une latence acceptable ;
8. couper proprement le streaming.

## Premier prototype à faire

Un CLI minimal :

```text
hue-spike --bridge 192.168.x.x --entertainment-area <id> --color red
```

Objectif :

- aucune UI ;
- aucune capture écran ;
- juste Hue REST + DTLS + HueStream.

Si ce prototype marche proprement, le projet est viable.

# Deuxième prototype : capture

Un CLI séparé :

```text
capture-spike
```

Objectif :

- demander l'autorisation ScreenCast via portal ;
- ouvrir le flux PipeWire ;
- récupérer des frames ;
- downscale ;
- calculer couleurs moyennes ;
- afficher FPS et latence.

Sortie possible :

```text
fps=30.0 capture=8ms analyze=2ms total=11ms
zone_0=#ff3300
zone_1=#2222ff
zone_2=#000000
zone_3=#ffaa00
```

# Troisième prototype : moteur sync

Fusion des deux prototypes :

```text
sync-spike
```

Objectif :

- capture écran ;
- analyse couleurs ;
- mapping zones ;
- streaming Hue ;
- métriques ;
- start/stop propre.

Toujours sans UI.

# Quatrième prototype : diagnostics

Un CLI séparé ou une sous-commande :

```text
lumaway doctor
```

Objectif :

- vérifier l'environnement local ;
- produire une sortie compréhensible ;
- fournir des hints concrets ;
- exporter un rapport JSON si nécessaire.

Exemple :

```text
[ok] portal.screencast: ScreenCast interface available
[ok] pipewire: PipeWire socket reachable
[warning] gstreamer.gl: GL color conversion plugin missing
[error] hue.dtls: DTLS handshake failed
hint: verify client_key and Entertainment Area activation
```

# Cinquième étape : UI native

Une fois le moteur validé :

- créer une app GTK4/libadwaita ;
- brancher le moteur sync ;
- afficher les métriques ;
- exposer les réglages essentiels.

# Interface recommandée

## Écran principal

Contenu :

- état Bridge ;
- état Entertainment Area ;
- état Capture ;
- état Sync ;
- bouton Start/Stop ;
- preview des zones ;
- FPS ;
- latence ;
- brightness ;
- smoothing.

## Assistant de configuration

Étapes :

1. découverte bridge ;
2. authentification ;
3. choix Entertainment Area ;
4. test couleur ;
5. sauvegarde.

## Préférences

Réglages MVP :

- FPS cible ;
- nombre de zones ;
- brightness scale ;
- smoothing ;
- gamma ;
- source écran/fenêtre si supporté ;
- reset permissions capture ;
- logs diagnostics ;
- export rapport diagnostic.

# Roadmap proposée

## Phase 0 : Spikes de risque

Objectif :

- valider Hue DTLS, capture Wayland, latence et diagnostics avant l'UI.

Livrables :

```text
examples/hue_fixed_color.rs
examples/capture_stats.rs
examples/sync_spike.rs
examples/doctor.rs
```

Critère de succès :

- les trois briques critiques fonctionnent hors UI ;
- les erreurs majeures sont diagnostiquées avec des messages actionnables.

## Phase 1 : Hue DTLS

Objectif :

- prouver le streaming Hue Entertainment en Rust.

Livrable :

```text
crates/lumaway-hue
examples/hue_fixed_color.rs
```

Critère de succès :

- une Entertainment Area change de couleur depuis Rust sans subprocess `openssl`.

## Phase 2 : Capture Wayland

Objectif :

- prouver la capture écran via portal/PipeWire.

Livrable :

```text
crates/lumaway-core
examples/capture_stats.rs
```

Critère de succès :

- capture stable à 30 FPS avec métriques.

## Phase 3 : Moteur sync

Objectif :

- relier capture, zones, couleurs et Hue.

Livrable :

```text
crates/lumaway-core/src/sync
```

Critère de succès :

- sync fonctionnelle sans UI pendant plusieurs minutes.

## Phase 4 : UI GTK

Objectif :

- créer l'application utilisable.

Livrable :

```text
crates/lumaway-gtk
```

Critère de succès :

- setup + start/stop + preview + métriques.

## Phase 5 : Packaging Flatpak

Objectif :

- distribuer proprement.

Livrable :

```text
build-aux/flatpak/
```

Critère de succès :

- build Flatpak local ;
- permissions portals correctes ;
- capture fonctionnelle dans sandbox ;
- diagnostic `doctor` pertinent dans et hors sandbox.

## Phase 6 : Polissage

Fonctions possibles :

- black bar detection ;
- profils ;
- mode lecture ;
- multi-monitor ;
- autostart ;
- tray icon si vraiment nécessaire ;
- diagnostics exportables ;
- calibration couleur ;
- mapping manuel des zones ;
- import depuis config Lumux.

## Phase 7 : Migration Lumux

Objectif :

- faciliter l'adoption par les utilisateurs existants sans copier la dette technique.

Import possible :

- bridge IP ;
- Entertainment Area sélectionnée si identifiable ;
- FPS cible ;
- smoothing ;
- brightness ;
- gamma ;
- nombre de zones ;
- préférences non sensibles.

À éviter :

- importer automatiquement `app_key` et `client_key` sans consentement explicite ;
- dépendre du format interne de Lumux comme contrat permanent ;
- masquer les secrets dans des logs ou rapports diagnostics.

# Invariants de conception

## Latence mesurée

Chaque frame doit pouvoir produire :

```text
capture_ms
analysis_ms
mapping_ms
send_ms
total_ms
fps
dropped_frames
```

## Runtime observable

L'utilisateur doit pouvoir savoir pourquoi ça ne marche pas :

- bridge inaccessible ;
- credentials invalides ;
- entertainment area inactive ;
- portal refusé ;
- PipeWire indisponible ;
- GStreamer plugin manquant ;
- DTLS échoué ;
- latence trop élevée ;
- bridge perdu pendant la sync ;
- permission portal révoquée ;
- session PipeWire fermée par l'environnement.

## Séparation UI / moteur

L'UI ne doit pas contenir la logique de sync.

Le moteur doit pouvoir tourner en CLI et être testé sans interface graphique.

## Backpressure explicite

Le moteur ne doit jamais accumuler une longue file de frames.

Règles :

- traiter la frame la plus récente ;
- abandonner les frames obsolètes ;
- mesurer les frames ignorées ;
- réduire temporairement le FPS cible si nécessaire ;
- ne pas bloquer l'UI sur le pipeline de sync.

## Secrets isolés

Les secrets Hue doivent être traités comme des credentials :

- stockage via Secret Service/libsecret/oo7 ;
- pas de logs contenant `app_key` ou `client_key` ;
- export diagnostic expurgé ;
- suppression possible depuis l'UI.

## Pas de dette inutile dans le MVP

Ne pas commencer par :

- un design complexe ;
- des animations ;
- un système de plugins ;
- une abstraction multi-OS ;
- un support complet des cas rares.

Commencer par le pipeline essentiel.

# Différences par rapport à Lumux

## Ce qu'il faut garder conceptuellement

- application Linux/Wayland ;
- Hue Entertainment ;
- capture via portal ;
- preview des zones ;
- settings simples ;
- Flatpak ;
- mode lecture plus tard.

## Ce qu'il faut éviter de recopier

- subprocess `openssl` comme mécanisme principal DTLS ;
- stockage brut des secrets Hue dans JSON ;
- mélange UI/core ;
- diagnostics via `print`;
- absence de tests ;
- logique temps réel dispersée ;
- dépendance trop forte aux callbacks UI.

# Verdict final

Pour une vraie alternative :

```text
LumaWay = Rust + GTK4/libadwaita + Flatpak + Linux Wayland
```

La première priorité n'est pas l'interface.

La première priorité est de prouver :

```text
Hue DTLS + capture Wayland + moteur sync stable
```

Une fois ces trois briques fiables, l'application peut devenir un vrai produit.
