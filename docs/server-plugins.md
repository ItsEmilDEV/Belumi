# Belumi Server – Plugin-Framework und Performance

Status: Phasen 1–4 und Profiler umgesetzt (2026-11-03). Entscheidungen: WASM-Plugins (wasmtime), dünne Hook-Patches im Horizon-Submodule.

## Ausgangslage (verifiziert)

- `vendor/horizon/crates/horizon-server` ist nur der Supervisor (`server.toml`, CLI, Backups, Updates, Panel).
- Der echte Server ist `horizon-client --dedicated`: `game/dedicated.rs` plus `lan/` (Hub, Chunks, Entities,
  Befehle, Chat). Er steckt im Client-Crate.
- Serverseitig gibt es keine Plugin-API. Vorhanden ist `game/native_hooks.rs` (leere Hook-Funktionen, API 1–4,
  u. a. `native_server_tick`, `native_world_lifecycle`) und Mixin-Bundles.

## Architektur

```
horizon-server (Supervisor, upstream)
   └─ horizon-client --dedicated (upstream + dünne Belumi-Hooks)
         └─ Hook-Aufrufe ──► belumi-server (Crate, Host-Seite)
                                ├─ EventBus (Filter nach Manifest-Subscription)
                                ├─ PluginManager (Laden, Reload, Abhängigkeiten)
                                └─ wasmtime-Instanzen (eine pro Plugin)
belumi-plugin-api  (Manifest, Events, ABI – von Host und SDKs geteilt)
```

Prinzip: Im Submodule stehen nur Einzeiler wie `belumi_hooks::emit(Event::PlayerJoin{..})`. Die Logik liegt in
den Belumi-Crates. So bleibt das Rebase auf Upstream billig.

## Plugin-Format

Ordner `plugins/<name>/` mit `plugin.toml` und `plugin.wasm` (siehe `PluginManifest`). Wichtige Felder:
`name`, `version`, `api_version` (muss `ABI_VERSION` entsprechen), `depends`, `subscribe`.

## ABI v1

Events laufen als JSON über die WASM-Grenze (einfach, versionierbar; binäres Format erst, wenn Messungen es verlangen).

Plugin-Exports:

| Export | Signatur | Zweck |
|---|---|---|
| `belumi_alloc` | `(len: i32) -> i32` | Host schreibt Event-JSON in den Plugin-Speicher |
| `belumi_on_event` | `(ptr: i32, len: i32) -> i32` | 0 = Continue, 1 = Cancel (nur bei cancellable Events) |
| `belumi_init` / `belumi_shutdown` | `() -> ()` | optional |

Host-Imports (Modul `belumi`), alle über eine Aktions-Queue: Plugins ändern nie direkt den Spielzustand, der Server
arbeitet die Queue zu Beginn jedes Ticks ab (max. 1024 Einträge, Überlauf wird verworfen):

| Import | Rückgabe | Permission |
|---|---|---|
| `broadcast(ptr, len)` | 0 ok, 1 verweigert, 2 Queue voll, -1 ungültiger Speicher | `messaging` |
| `send_message(uuid_ptr, uuid_len, ptr, len)` | wie oben | `messaging` |
| `kick(uuid_ptr, uuid_len, reason_ptr, reason_len)` | wie oben | `kick` |
| `player_count() -> i32` | Spielerzahl (aktualisiert bei Join/Quit) | – |
| `players(ptr, cap) -> i32` | schreibt `[{"uuid","name"}]` als JSON in den Puffer; Rückgabe = nötige Länge (größer als `cap`: nichts geschrieben, mit größerem Puffer wiederholen), -1 Fehler | – |
| `log(level, ptr, len)` | – | – |

Berechtigungen stehen im Manifest (`permissions = ["messaging", "kick"]`).

Sicherheit: Fuel/Epoch-Limit pro Aufruf, Speicherlimit pro Instanz, ein Panic/Trap deaktiviert nur das Plugin.

## Starten und konfigurieren

```sh
belumi-server --dir ./myserver --client <horizon-client mit Feature belumi> --resources <.local/horizon-client/26.3>
```

`belumi-server` legt beim ersten Start `belumi.toml` und `plugins/` an und ersetzt sich dann durch
`horizon-client --dedicated --nogui` im Serverordner (stdin `stop`, Signale und Konsole sind die des Spiels).
`belumi.toml`: `[server] client/resources`, `[tuning] spawn_interval, profile_seconds`.
Der Server meldet sich im Brand-Paket als `Belumi` (F3 zeigt `"Belumi" server`).

## Plugin-Befehle

`commands = ["hello"]` im Manifest. Der Server nimmt sie als `/hello [args]` in den Command-Tree auf (ein
greedy-Textargument, also mit Tab-Vervollständigung des Namens), und `Event::PluginCommand` geht nur an das
besitzende Plugin. Ein Plugin-Befehl gewinnt gegen einen eingebauten gleichen Namens.

**Berechtigungsstufen:** `[command_levels]` im Manifest setzt die Mindest-Operatorstufe (0–4) je Befehl, Standard 0:

```toml
commands = ["hello", "ban"]
[command_levels]
ban = 3
```

Zu niedrige Stufe: der Spieler bekommt „You do not have permission…“, das Plugin sieht nichts. Das Manifest wird beim Laden
geprüft (unbekannter Befehl oder Stufe über 4 = Fehler).

Das Beispiel `examples/hello-plugin` zeigt `host::player_list()` (`/players`) und einen Befehl mit Stufe 2 (`/shout`).

**Reload:** `/belumi reload` (Stufe 4) schickt `server_stop` an alle Plugins, liest `plugins/` neu und schickt `server_start`.

**Events:** `server_start`, `server_stop`, `tick`, `player_join`, `player_quit`, `chat` (abbrechbar), `command` (abbrechbar),
`block_break` (abbrechbar), `use_block` (Rechtsklick auf einen Block, abbrechbar; Position, Seite 0–5, Hand), `player_death`, `plugin_command`.

## Profiler und Performance

`BELUMI_PROFILE=SECONDS` (oder `profile_seconds`) loggt je Zeitraum: Ticks, mittlerer und schlimmster Tick, Ticks über 50 ms
und die teuersten Spans (Mittel/Max pro Tick). Gemessen mit `horizon-bots` (20 Bots, Flachwelt, Fast-Profil):

| Ausgangslage (Ø Tick ca. 10–11 ms) | Anteil |
|---|---|
| `spawn.attempts` (natürliches Spawnen) | ca. 50 % |
| `creatures.entities` (Mob-Ticks) | ca. 20 % |
| `tick_lan` (Netzwerk/Spieler) | ca. 13 % |
| Chunks senden (`lan_chunks`), Streaming, Blöcke | je unter 3 % |

Folgerung: Bei dieser Last lohnt nur das Spawnen. Chunk-Senden, Entity-Tracking und Netzwerk sind nicht heiß, und ich ändere sie
ohne Messung nicht. Umgesetzt:

- **`spawn_interval = N`** (`BELUMI_SPAWN_INTERVAL`): natürliche Spawn-Versuche nur jeden N-ten Tick (wie Paper `ticks-per.*-spawns`);
  Standard 1 = Vanilla. 20 Bots, gleicher Lauf: Ø Tick 11,6 ms (N=1), 7,8 ms (N=2), 5,2 ms (N=4). Die Spawnrate sinkt um denselben
  Faktor, der 400-Tick-Tierdurchlauf läuft immer. Die Läufe sind nicht identisch, weil weniger Spawns auch weniger Mobs bedeuten.

### Zweite Messrunde: realistische Last

40 wandernde Bots (`explore`/`walk`), normale Welt, View-Distance 10, Simulation 8, Fast-Profil. Der Tick wächst mit den geladenen
Chunks und Mobs von 7 ms auf über 50 ms nach etwa 40 s. Neue Spans (nur aktiv mit `BELUMI_PROFILE`) zeigen, wo die Zeit liegt
(Mittel am Ende des Laufs, schwankt mit der Maschinenlast um ±15 %):

| Span | ms/Tick | Anteil |
|---|---|---|
| `creatures.entities` (davon `ent.mob_tick` ca. 60 %, `ent.setup` ca. 8 %) | 12–16 | ca. 35 % |
| `tick_lan` (`hub.tick_level`, `lan.entities`, `lan_chunks`) | 6–9 | ca. 20 % |
| `tick_blocks` (`blocks.scheduled` ca. 4, `blocks.random` ca. 3) | 7–9 | ca. 20 % |
| `streaming` (Chunk-Eingang und Licht; Spitzen bis 30 ms) | 7–9 | ca. 20 % |
| `spawn.attempts` | 3–5 | ca. 10 % |

Gefunden und behoben (Verhalten unverändert, Tests grün):

- `species::def` suchte linear durch alle Arten, `is_spawn_position_ok` baute pro Versuch einen `format!`-String. Beides läuft rund
  10 000-mal pro Tick (20 Bots, Flachwelt). Jetzt Index-Tabelle und einmal gebaute IDs. `pos_ok` im Spawn-Pfad: minus 60 %, der Tick
  insgesamt minus 5 %, in der ganzen Mob-Logik wirkt `def` zusätzlich.

Bewusst nicht gebaut (Entscheidung „gameplay-neutral“): Aktivierungsradien und Spawn-Raten wie bei Paper/Purpur. Der Vanilla-Algorithmus
ist im Horizon-Code schon sorgfältig optimiert (Gitter für Mob-Boxen, Nearest-Cache). Ein Vergleich mit Paper/Purpur auf derselben Last
steht aus; ohne ihn behaupte ich keinen Faktor.

Methodik-Hinweis: `gdb`-Sampling per Signal taugt hier nicht (der Haupt-Thread wird fast nur im Schlaf unterbrochen), die Spans sind
die belastbare Quelle. Kandidaten als Nächstes: `ent.mob_tick` pro Art aufschlüsseln, `blocks.scheduled` (Flüssigkeiten in frischen
Chunks), Licht-Anwendung im Streaming auf Worker verschieben, Autosave in Scheiben.

## Phasen

1. **Phase 1 (erledigt):** `belumi-plugin-api`.
2. **Phase 2 (erledigt):** `belumi-server` mit wasmtime-Host (Fuel/Speicherlimits, Abhängigkeiten, Subscriptions).
3. **Phase 3 (erledigt):** Hooks im Submodule (Start, Stop, Tick, Join, Quit, Chat, Command, BlockBreak).
4. **Phase 4 (erledigt):** Host-Imports mit Berechtigungen, Aktions-Queue, Plugin-Befehle im Command-Tree, Rust-SDK.
5. **Phase 5 (begonnen):** Tick-Profiler, `spawn_interval`. Weitere Tweaks nach Messung (siehe oben).

## Offen

- Plugin-Hooks für Blockplatzieren als eigenes Event, Respawn und Weltwechsel (`use_block` deckt Platzieren und Benutzen ab).
- Performance: siehe Abschnitt Profiler und Performance.
