# nrzPlan – justfile (cargo install just)
set dotenv-load := true

default:
    @just --list

# Dev-Server (Web + Server, http://127.0.0.1:8080)
dev:
    pnpm css:build
    dx serve

# Tailwind im Watch-Modus (zweites Terminal)
css:
    pnpm css

# Release-Build (Web + Server)
bundle:
    pnpm css:build
    dx bundle --release

# Tests
test:
    cargo test --no-default-features --features server

# Datenbank zurücksetzen (löscht data/nrzplan.db)
reset-db:
    rm -f data/nrzplan.db data/nrzplan.db-shm data/nrzplan.db-wal
