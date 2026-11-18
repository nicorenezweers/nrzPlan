# nrzPlan - Umbrel-Paket

Dieser Zweig enthaelt nur fertige Dateien fuer die App `nrz-plan` im nrzStore:

| Datei | Inhalt |
|-------|--------|
| `nrzplan-amd64.gz`, `nrzplan-arm64.gz` | Server (Linux, glibc 2.31) |
| `public.tar.gz` | Oberflaeche (WASM, CSS, Icons) |
| `SHA256SUMS` | Pruefsummen |

`hooks/pre-start` der App laedt sie per Tag `umbrel-<version>`. Erstellt mit
`scripts/umbrel-release.sh` auf dem Zweig `main`.
