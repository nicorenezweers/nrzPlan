use dioxus::prelude::*;
use icons::{Copy, KeyRound, Trash2};

use crate::api::{create_api_key, delete_api_key, get_api_keys};
use crate::components::hooks::use_theme_mode::use_theme_mode;
use crate::components::layout::page::{PageHeader, Section};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::INPUT;
use crate::domain::boards::card_dialog::copy_to_clipboard;
use crate::domain::settings::integrations::{IcsSection, WebhooksSection};

const WEBHOOK_EXAMPLE: &str = r#"POST https://deine-url
X-NrzPlan-Event: card.moved
X-NrzPlan-Signature: sha256=…

{
  "event": "card.moved",
  "message": "„GitHub integration“: Geplant → In Arbeit",
  "actor": "API · GitHub Actions",
  "at": "2026-11-14T09:12:00",
  "board_id": "…",
  "data": { "card": { … }, "from": "Geplant", "to": "In Arbeit" }
}"#;

fn key_meta(k: &crate::model::ApiKey) -> String {
    let used = match k.last_used_at {
        Some(t) => format!("zuletzt {}", t.format("%d.%m.%Y %H:%M")),
        None => "nie benutzt".into(),
    };
    format!("{}… · erstellt {} · {used}", k.prefix, k.created_at.format("%d.%m.%Y"))
}

#[component]
fn CodeBlock(#[props(into)] title: String, #[props(into)] code: String) -> Element {
    let c = code.clone();
    rsx! {
        div { class: "overflow-hidden rounded-lg border",
            div { class: "flex items-center justify-between border-b bg-card/60 px-3 py-1.5",
                span { class: "text-xs text-muted-foreground", "{title}" }
                button { class: "text-muted-foreground hover:text-foreground", title: "Kopieren", onclick: move |_| copy_to_clipboard(c.clone()),
                    Copy { class: "size-3.5" }
                }
            }
            pre { class: "overflow-x-auto bg-background/60 p-3 font-mono text-xs leading-relaxed", "{code}" }
        }
    }
}

#[component]
pub fn SettingsPage() -> Element {
    let mut keys = use_server_future(get_api_keys)?;
    let mut name = use_signal(String::new);
    let mut secret = use_signal(|| None::<String>);
    let mut origin = use_signal(|| "http://localhost:8080".to_string());
    let mut is_dark = use_theme_mode();

    use_effect(move || {
        spawn(async move {
            if let Ok(serde_json::Value::String(o)) = document::eval("return location.origin;").await {
                origin.set(o);
            }
        });
    });

    let create = move |e: FormEvent| {
        e.prevent_default();
        let n = name().trim().to_string();
        let n = if n.is_empty() { "API-Key".to_string() } else { n };
        spawn(async move {
            if let Ok(k) = create_api_key(n).await {
                secret.set(Some(k.secret));
                name.set(String::new());
                keys.restart();
            }
        });
    };

    let o = origin();
    let key_hint = secret().unwrap_or_else(|| "$NRZPLAN_KEY".into());
    let auth = format!("-H \"Authorization: Bearer {key_hint}\"");
    let ex_card = format!(
        "curl -X POST {o}/api/v1/boards/Roadmap/cards \\\n  {auth} \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"title\": \"GitHub integration\", \"status\": \"Geplant\", \"labels\": [\"Feature\"],\n       \"source\": \"github\", \"external_id\": \"42\"}}'"
    );
    let ex_move = format!(
        "curl -X PATCH {o}/api/v1/boards/Roadmap/cards/external/github/42 \\\n  {auth} \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"status\": \"In Arbeit\"}}'"
    );
    let ex_todo = format!(
        "curl -X POST {o}/api/v1/todos \\\n  {auth} \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"title\": \"Rechnung bezahlen\", \"due_date\": \"2026-11-20\", \"priority\": 2, \"list\": \"Privat\"}}'"
    );
    let ex_event = format!(
        "curl -X POST {o}/api/v1/events \\\n  {auth} \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"title\": \"Zahnarzt\", \"start_at\": \"2026-11-18T09:30:00\", \"end_at\": \"2026-11-18T10:15:00\"}}'"
    );
    let ex_agenda = format!("curl {o}/api/v1/agenda {auth}");
    let ex_recurring = format!(
        "curl -X POST {o}/api/v1/todos \\\n  {auth} \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"title\": \"Wochenreview\", \"recurrence\": \"weekly\", \"due_date\": \"2026-11-20\"}}'"
    );

    rsx! {
        div { class: "mx-auto flex w-full max-w-3xl flex-col",
            PageHeader { title: "Einstellungen", subtitle: "Darstellung, API-Keys und Integrationen" }
            div { class: "flex flex-col gap-8 px-4 pb-12 sm:px-8",
                Section { title: "Darstellung",
                    div { class: "flex items-center justify-between rounded-xl border bg-card/40 px-4 py-3",
                        div {
                            p { class: "text-sm font-medium", "Dunkles Thema" }
                            p { class: "text-xs text-muted-foreground", "nrz-Standard. Hell nur, wenn ausdrücklich gewählt." }
                        }
                        button {
                            class: if is_dark() { "relative h-6 w-11 rounded-full bg-primary transition-colors" } else { "relative h-6 w-11 rounded-full bg-muted transition-colors" },
                            onclick: move |_| is_dark.toggle(),
                            span { class: if is_dark() { "absolute left-[22px] top-0.5 size-5 rounded-full bg-primary-foreground transition-all" } else { "absolute left-0.5 top-0.5 size-5 rounded-full bg-foreground transition-all" } }
                        }
                    }
                }

                Section { title: "API-Keys",
                    p { class: "px-1 pb-2 text-sm text-muted-foreground",
                        "Mit einem API-Key können externe Tools (Skripte, CI, n8n, Shortcuts, Home Assistant …) Karten anlegen, Status ändern, Todos und Termine verwalten."
                    }
                    form { class: "flex gap-2", onsubmit: create,
                        input { class: INPUT, placeholder: "Name, z.B. GitHub Actions", value: name(), oninput: move |e| name.set(e.value()) }
                        Button { submit: true, KeyRound { class: "size-4" } "Key erzeugen" }
                    }
                    if let Some(s) = secret() {
                        div { class: "mt-2 flex flex-col gap-2 rounded-xl border border-emerald-500/40 bg-emerald-500/10 p-3",
                            p { class: "text-xs text-emerald-300", "Neuer Key – nur jetzt sichtbar, bitte sicher ablegen:" }
                            div { class: "flex items-center gap-2",
                                code { class: "flex-1 break-all rounded bg-background/60 px-2 py-1 font-mono text-xs", "{s}" }
                                Button {
                                    variant: ButtonVariant::Outline,
                                    size: ButtonSize::Sm,
                                    onclick: {
                                        let s = s.clone();
                                        move |_| copy_to_clipboard(s.clone())
                                    },
                                    Copy { class: "size-3.5" }
                                    "Kopieren"
                                }
                            }
                        }
                    }
                    div { class: "mt-2 flex flex-col divide-y rounded-xl border",
                        match &*keys.read() {
                            Some(Ok(list)) if list.is_empty() => rsx! {
                                p { class: "px-4 py-3 text-sm text-muted-foreground", "Noch keine Keys." }
                            },
                            Some(Ok(list)) => rsx! {
                                for k in list.iter().cloned() {
                                    div { key: "{k.id}", class: "flex items-center gap-3 px-4 py-2.5",
                                        KeyRound { class: "size-4 text-muted-foreground" }
                                        div { class: "min-w-0 flex-1",
                                            p { class: "truncate text-sm font-medium", "{k.name}" }
                                            p { class: "font-mono text-[11px] text-muted-foreground",
                                                {key_meta(&k)}
                                            }
                                        }
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            size: ButtonSize::Icon,
                                            class: "text-muted-foreground hover:text-destructive",
                                            onclick: move |_| {
                                                let id = k.id.clone();
                                                spawn(async move {
                                                    let ok = document::eval("return confirm('Key widerrufen?');").await.ok().and_then(|v| v.as_bool()).unwrap_or(false);
                                                    if ok && delete_api_key(id).await.is_ok() {
                                                        keys.restart();
                                                    }
                                                });
                                            },
                                            Trash2 { class: "size-4" }
                                        }
                                    }
                                }
                            },
                            Some(Err(e)) => rsx! { p { class: "px-4 py-3 text-sm text-destructive", "Fehler: {e}" } },
                            None => rsx! {},
                        }
                    }
                }

                IcsSection { origin: o.clone() }
                WebhooksSection {}

                Section { title: "REST-API",
                    div { class: "flex flex-col gap-3 text-sm text-muted-foreground",
                        p {
                            "Basis-URL "
                            code { class: "rounded bg-muted px-1 font-mono text-xs text-foreground", "{o}/api/v1" }
                            " · Auth per "
                            code { class: "rounded bg-muted px-1 font-mono text-xs text-foreground", "Authorization: Bearer <key>" }
                            " oder "
                            code { class: "rounded bg-muted px-1 font-mono text-xs text-foreground", "X-API-Key" }
                            ". Boards und Spalten können per ID "
                            em { "oder Name" }
                            " angesprochen werden. Mit "
                            code { class: "rounded bg-muted px-1 font-mono text-xs text-foreground", "source" }
                            " + "
                            code { class: "rounded bg-muted px-1 font-mono text-xs text-foreground", "external_id" }
                            " werden Einträge idempotent synchronisiert (erneutes POST aktualisiert statt dupliziert)."
                        }
                        CodeBlock { title: "Externe Karte anlegen (z.B. aus GitHub-Issue)", code: ex_card }
                        CodeBlock { title: "Status ändern (Karte verschieben)", code: ex_move }
                        CodeBlock { title: "Todo anlegen", code: ex_todo }
                        CodeBlock { title: "Termin anlegen", code: ex_event }
                        CodeBlock { title: "Tagesübersicht abrufen", code: ex_agenda }
                        CodeBlock { title: "Wiederkehrendes Todo", code: ex_recurring }
                        CodeBlock { title: "Webhook-Payload (Beispiel)", code: WEBHOOK_EXAMPLE }
                        p {
                            "Alle Endpunkte: "
                            a { class: "underline hover:text-foreground", href: "/api/v1", target: "_blank", "GET /api/v1" }
                        }
                    }
                }
            }
        }
    }
}
