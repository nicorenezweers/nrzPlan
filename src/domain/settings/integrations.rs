use dioxus::prelude::*;
use icons::{CalendarDays, Copy, RotateCcw, Send, Trash2, Webhook as WebhookIcon};

use crate::api::{create_webhook, delete_webhook, get_ics_token, get_webhook_events, get_webhooks, set_webhook_active, test_webhook};
use crate::components::layout::page::Section;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{Field, INPUT};
use crate::domain::boards::card_dialog::copy_to_clipboard;
use crate::model::{NewWebhook, Webhook};
use crate::util::fmt_ago;

fn status_line(h: &Webhook) -> (String, &'static str) {
    match (h.last_status, &h.last_error, h.last_at) {
        (_, Some(err), Some(at)) => (format!("Fehler: {err} · {}", fmt_ago(at)), "text-red-400"),
        (Some(s), None, Some(at)) => (format!("HTTP {s} · {}", fmt_ago(at)), "text-emerald-400"),
        _ => ("Noch nicht ausgelöst".into(), "text-muted-foreground"),
    }
}

/// Ausgehende Webhooks: externe Systeme über Änderungen informieren.
#[component]
pub fn WebhooksSection() -> Element {
    let mut hooks = use_resource(get_webhooks);
    let kinds = use_resource(get_webhook_events);
    let mut adding = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut url = use_signal(String::new);
    let mut secret = use_signal(String::new);
    let mut events = use_signal(Vec::<String>::new);
    let mut error = use_signal(|| None::<String>);
    let mut test_result = use_signal(|| None::<(String, String)>);

    let save = move |e: FormEvent| {
        e.prevent_default();
        let input = NewWebhook { name: name(), url: url(), secret: Some(secret()), events: events() };
        spawn(async move {
            match create_webhook(input).await {
                Ok(_) => {
                    adding.set(false);
                    name.set(String::new());
                    url.set(String::new());
                    secret.set(String::new());
                    events.set(Vec::new());
                    error.set(None);
                    hooks.restart();
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    let all_kinds: Vec<String> = kinds.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default();

    rsx! {
        Section { title: "Webhooks",
            p { class: "px-1 pb-2 text-sm text-muted-foreground",
                "nrzPlan schickt bei Änderungen (Karte verschoben, Todo erledigt, …) ein JSON-POST an deine URL – z.B. für n8n, Home Assistant, Discord oder eigene Skripte. Mit Secret wird jede Anfrage per "
                code { class: "rounded bg-muted px-1 font-mono text-xs text-foreground", "X-NrzPlan-Signature: sha256=…" }
                " (HMAC) signiert."
            }
            div { class: "flex flex-col divide-y rounded-xl border",
                match &*hooks.read() {
                    Some(Ok(list)) if list.is_empty() => rsx! {
                        p { class: "px-4 py-3 text-sm text-muted-foreground", "Noch keine Webhooks." }
                    },
                    Some(Ok(list)) => rsx! {
                        for h in list.iter().cloned() {
                            {
                                let (status, tone) = status_line(&h);
                                let id_toggle = h.id.clone();
                                let id_test = h.id.clone();
                                let id_del = h.id.clone();
                                let active = h.active;
                                let events_label = if h.events.is_empty() { "alle Ereignisse".to_string() } else { h.events.join(", ") };
                                rsx! {
                                    div { key: "{h.id}", class: "flex flex-col gap-1.5 px-4 py-3",
                                        div { class: "flex items-center gap-3",
                                            WebhookIcon { class: "size-4 shrink-0 text-muted-foreground" }
                                            div { class: "min-w-0 flex-1",
                                                p { class: if active { "truncate text-sm font-medium" } else { "truncate text-sm font-medium text-muted-foreground line-through" }, "{h.name}" }
                                                p { class: "truncate font-mono text-[11px] text-muted-foreground", "{h.url}" }
                                            }
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::Sm,
                                                onclick: move |_| {
                                                    let id = id_test.clone();
                                                    spawn(async move {
                                                        let msg = test_webhook(id.clone()).await.unwrap_or_else(|e| e.to_string());
                                                        test_result.set(Some((id, msg)));
                                                        hooks.restart();
                                                    });
                                                },
                                                Send { class: "size-3.5" }
                                                "Test"
                                            }
                                            button {
                                                class: if active { "relative h-5 w-9 shrink-0 rounded-full bg-primary" } else { "relative h-5 w-9 shrink-0 rounded-full bg-muted" },
                                                title: if active { "Deaktivieren" } else { "Aktivieren" },
                                                onclick: move |_| {
                                                    let id = id_toggle.clone();
                                                    spawn(async move {
                                                        let _ = set_webhook_active(id, !active).await;
                                                        hooks.restart();
                                                    });
                                                },
                                                span { class: if active { "absolute left-[18px] top-0.5 size-4 rounded-full bg-primary-foreground transition-all" } else { "absolute left-0.5 top-0.5 size-4 rounded-full bg-foreground transition-all" } }
                                            }
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::Icon,
                                                class: "size-8 text-muted-foreground hover:text-destructive",
                                                onclick: move |_| {
                                                    let id = id_del.clone();
                                                    spawn(async move {
                                                        let ok = document::eval("return confirm('Webhook löschen?');").await.ok().and_then(|v| v.as_bool()).unwrap_or(false);
                                                        if ok && delete_webhook(id).await.is_ok() {
                                                            hooks.restart();
                                                        }
                                                    });
                                                },
                                                Trash2 { class: "size-4" }
                                            }
                                        }
                                        p { class: "pl-7 text-[11px] text-muted-foreground", "{events_label}" }
                                        p { class: "pl-7 text-[11px] {tone}", "{status}" }
                                        if let Some((tid, msg)) = test_result().filter(|(tid, _)| *tid == h.id) {
                                            p { key: "{tid}", class: "pl-7 text-[11px] text-foreground", "Test: {msg}" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { p { class: "px-4 py-3 text-sm text-destructive", "Fehler: {e}" } },
                    None => rsx! {},
                }
            }
            if adding() {
                form { class: "mt-2 flex flex-col gap-3 rounded-xl border bg-card/40 p-4", onsubmit: save,
                    div { class: "grid gap-3 sm:grid-cols-2",
                        Field { label: "Name",
                            input { class: INPUT, placeholder: "z.B. n8n", value: name(), oninput: move |e| name.set(e.value()) }
                        }
                        Field { label: "Secret (optional)",
                            input { class: INPUT, placeholder: "für HMAC-Signatur", value: secret(), oninput: move |e| secret.set(e.value()) }
                        }
                    }
                    Field { label: "URL",
                        input { class: INPUT, placeholder: "https://…", value: url(), oninput: move |e| url.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        span { class: "text-xs font-medium text-muted-foreground", "Ereignisse (keine Auswahl = alle)" }
                        div { class: "flex flex-wrap gap-1.5",
                            for k in all_kinds {
                                {
                                    let on = events().contains(&k);
                                    let k2 = k.clone();
                                    rsx! {
                                        button {
                                            key: "{k}",
                                            r#type: "button",
                                            class: if on { "rounded-full border border-foreground/40 bg-accent px-2.5 py-0.5 font-mono text-[11px]" } else { "rounded-full border px-2.5 py-0.5 font-mono text-[11px] text-muted-foreground hover:text-foreground" },
                                            onclick: move |_| {
                                                let mut ev = events.write();
                                                if let Some(pos) = ev.iter().position(|x| *x == k2) { ev.remove(pos); } else { ev.push(k2.clone()); }
                                            },
                                            "{k}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(err) = error() {
                        p { class: "text-sm text-destructive", "{err}" }
                    }
                    div { class: "flex gap-2",
                        Button { submit: true, "Speichern" }
                        Button { variant: ButtonVariant::Ghost, onclick: move |_| adding.set(false), "Abbrechen" }
                    }
                }
            } else {
                div { class: "mt-2",
                    Button { variant: ButtonVariant::Outline, onclick: move |_| adding.set(true),
                        WebhookIcon { class: "size-4" }
                        "Webhook hinzufügen"
                    }
                }
            }
        }
    }
}

/// Abo-Link für Apple/Google/Outlook-Kalender.
#[component]
pub fn IcsSection(origin: String) -> Element {
    let mut token = use_signal(|| None::<String>);
    let rotate = move |r: bool| {
        spawn(async move {
            if let Ok(t) = get_ics_token(r).await {
                token.set(Some(t));
            }
        });
    };
    let feed = token().map(|t| format!("{origin}/api/v1/calendar.ics?token={t}"));
    let webcal = feed.as_ref().map(|f| f.replacen("https://", "webcal://", 1).replacen("http://", "webcal://", 1));

    rsx! {
        Section { title: "Kalender-Abo",
            p { class: "px-1 pb-2 text-sm text-muted-foreground",
                "Termine, fällige Todos und Board-Karten in Apple Kalender, Google Kalender oder Outlook abonnieren (aktualisiert alle 15 Minuten, nur lesend)."
            }
            match feed {
                Some(f) => rsx! {
                    div { class: "flex flex-col gap-2 rounded-xl border bg-card/40 p-3",
                        code { class: "break-all rounded bg-background/60 px-2 py-1.5 font-mono text-xs", "{f}" }
                        div { class: "flex flex-wrap gap-2",
                            Button {
                                variant: ButtonVariant::Outline,
                                size: ButtonSize::Sm,
                                onclick: {
                                    let f = f.clone();
                                    move |_| copy_to_clipboard(f.clone())
                                },
                                Copy { class: "size-3.5" }
                                "Link kopieren"
                            }
                            if let Some(w) = webcal {
                                a { class: "inline-flex h-8 items-center gap-1.5 rounded-md border px-3 text-sm hover:bg-accent", href: "{w}",
                                    CalendarDays { class: "size-3.5" }
                                    "In Kalender öffnen"
                                }
                            }
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Sm,
                                class: "text-muted-foreground",
                                onclick: move |_| {
                                    spawn(async move {
                                        let ok = document::eval("return confirm('Neuen Link erzeugen? Bestehende Abos funktionieren dann nicht mehr.');")
                                            .await
                                            .ok()
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false);
                                        if ok {
                                            rotate(true);
                                        }
                                    });
                                },
                                RotateCcw { class: "size-3.5" }
                                "Neu erzeugen"
                            }
                        }
                        p { class: "text-[11px] text-muted-foreground", "Der Link enthält ein geheimes Token – nicht öffentlich teilen." }
                    }
                },
                None => rsx! {
                    div {
                        Button { variant: ButtonVariant::Outline, onclick: move |_| rotate(false),
                            CalendarDays { class: "size-4" }
                            "Abo-Link anzeigen"
                        }
                    }
                },
            }
        }
    }
}
