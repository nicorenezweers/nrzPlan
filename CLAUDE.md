# nrzPlan

Dioxus 0.7 fullstack + SQLite, UI aus nrzUI (dunkles nrz-Thema, nur Theme-Variablen verwenden).

- Server prüfen: `cargo check --no-default-features --features server`; Client: `cargo check --target wasm32-unknown-unknown --features web`.
- Tailwind: `pnpm css:build` nach neuen Klassen (Quelle `style/tailwind.css` → `public/tailwind.css`).
- DB-Logik nur in `src/server/db.rs`; UI (`src/api.rs`) und REST (`src/server/rest.rs`) rufen sie auf.
- Zeitstempel in der DB immer über `ts()` (`%Y-%m-%dT%H:%M:%S`), sonst stimmen Bereichsabfragen nicht.
- Buttons in Formularen: `submit: true` nur für den Absenden-Button.
- Änderungen an Karten/Todos/Terminen in `db.rs` immer mit `events::emit` melden (Aktivitätslog + Webhooks).
- REST-Auth läuft als Middleware und setzt den Akteur (`events::ACTOR`); neue geschützte Routen in `protected` eintragen.
- Daten, die die ⌘K-Palette ändert: Seiten lesen `Refresh` im `use_server_future`, damit sie neu laden.
- Umbrel-Release nur mit `scripts/umbrel-release.sh` (dx braucht für den Server `@server ... --platform server`, sonst landet dioxus-web im Server und die Hydration bricht).
- Paketdateien für den nrzStore in `umbrel/` (App `nrz-plan`, Port 8560); `nrzStore.json` nicht umbenennen.
