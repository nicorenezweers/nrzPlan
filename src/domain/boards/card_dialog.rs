use dioxus::prelude::*;
use icons::{Copy, ExternalLink, Plus, Trash2, X, Zap};

use crate::api::{add_checklist_item, delete_card, delete_checklist_item, get_card, set_checklist_item, update_card};
use crate::components::ui::badge::LabelBadge;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{Field, GHOST_INPUT, INPUT, SELECT, TEXTAREA};
use crate::components::ui::modal::Modal;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{CardDetail, CardPatch, Column, Label};
use crate::util::{date_input, parse_date, priority_label};

pub fn copy_to_clipboard(text: String) {
    let js = format!("navigator.clipboard && navigator.clipboard.writeText({});", serde_json::json!(text));
    document::eval(&js);
}

/// Detail-Dialog einer Karte. `onclose(true)` = Board neu laden.
#[component]
pub fn CardDialog(card_id: String, columns: Vec<Column>, labels: Vec<Label>, onclose: EventHandler<bool>) -> Element {
    let id = card_id.clone();
    let detail = use_resource(move || get_card(id.clone()));
    let mut changed = use_signal(|| false);

    let content = match &*detail.read() {
        Some(Ok(d)) => rsx! {
            CardForm {
                key: "{d.card.id}",
                detail: d.clone(),
                columns: columns.clone(),
                labels: labels.clone(),
                on_changed: move |_| changed.set(true),
                onclose: move |reload: bool| onclose.call(reload || changed()),
                detail_res: detail,
            }
        },
        Some(Err(e)) => rsx! { p { class: "p-5 text-sm text-destructive", "Fehler: {e}" } },
        None => rsx! {
            div { class: "flex flex-col gap-3 p-5",
                Skeleton { class: "h-7 w-2/3" }
                Skeleton { class: "h-24 w-full" }
            }
        },
    };

    rsx! {
        Modal { class: "max-w-2xl", onclose: move |_| onclose.call(changed()), {content} }
    }
}

#[component]
fn CardForm(
    detail: CardDetail,
    columns: Vec<Column>,
    labels: Vec<Label>,
    on_changed: EventHandler<()>,
    onclose: EventHandler<bool>,
    detail_res: Resource<Result<CardDetail, ServerFnError>>,
) -> Element {
    let card = detail.card.clone();
    let mut title = use_signal(|| card.title.clone());
    let mut description = use_signal(|| card.description.clone().unwrap_or_default());
    let mut column = use_signal(|| card.column_id.clone());
    let mut card_labels = use_signal(|| card.labels.clone());
    let mut new_label = use_signal(String::new);
    let mut priority = use_signal(|| card.priority);
    let mut due = use_signal(|| date_input(card.due_date));
    let mut url = use_signal(|| card.url.clone().unwrap_or_default());
    let mut new_item = use_signal(String::new);
    let mut detail_res = detail_res;

    let id = card.id.clone();
    let save = {
        let id = id.clone();
        move |_| {
            let id = id.clone();
            let patch = CardPatch {
                title: Some(title()),
                description: Some(Some(description())),
                column: Some(column()),
                labels: Some(card_labels()),
                priority: Some(priority()),
                due_date: Some(parse_date(&due())),
                url: Some(Some(url())),
                position: None,
            };
            spawn(async move {
                if update_card(id, patch).await.is_ok() {
                    onclose.call(true);
                }
            });
        }
    };

    let add_item = {
        let id = id.clone();
        move |e: FormEvent| {
            e.prevent_default();
            let text = new_item().trim().to_string();
            if text.is_empty() {
                return;
            }
            let id = id.clone();
            spawn(async move {
                if add_checklist_item(id, text).await.is_ok() {
                    new_item.set(String::new());
                    on_changed.call(());
                    detail_res.restart();
                }
            });
        }
    };

    let mut add_label = move || {
        let l = new_label().trim().trim_start_matches('#').to_string();
        if !l.is_empty() && !card_labels().iter().any(|x| x.eq_ignore_ascii_case(&l)) {
            card_labels.write().push(l);
        }
        new_label.set(String::new());
    };

    let available: Vec<Label> =
        labels.iter().filter(|l| !card_labels().iter().any(|c| c.eq_ignore_ascii_case(&l.name))).cloned().collect();
    let done_count = detail.checklist.iter().filter(|i| i.done).count();
    let api_path = format!("/api/v1/cards/{}", card.id);
    let id_del = card.id.clone();
    let created = card.created_at.format("%d.%m.%Y %H:%M").to_string();

    rsx! {
        div { class: "flex flex-col gap-5 p-5",
            div { class: "flex items-start gap-3",
                div { class: "flex-1",
                    p { class: "mb-1 text-xs text-muted-foreground", "{detail.board_name} / {detail.column_name}" }
                    input {
                        class: "{GHOST_INPUT} text-lg font-semibold",
                        value: title(),
                        oninput: move |e| title.set(e.value()),
                    }
                }
                button {
                    class: "rounded-md p-1 text-muted-foreground hover:bg-accent hover:text-foreground",
                    onclick: move |_| onclose.call(false),
                    X { class: "size-4" }
                }
            }

            div { class: "grid grid-cols-2 gap-3 sm:grid-cols-3",
                Field { label: "Status",
                    select { class: SELECT, onchange: move |e| column.set(e.value()),
                        for c in columns.iter() {
                            option { value: "{c.id}", selected: c.id == column(), "{c.name}" }
                        }
                    }
                }
                Field { label: "Priorität",
                    select { class: SELECT, onchange: move |e| priority.set(e.value().parse().unwrap_or(0)),
                        for p in [0i64, 1, 2, 3] {
                            option { value: "{p}", selected: p == priority(), "{priority_label(p)}" }
                        }
                    }
                }
                Field { label: "Fällig",
                    input { r#type: "date", class: INPUT, value: due(), oninput: move |e| due.set(e.value()) }
                }
            }

            Field { label: "Labels",
                div { class: "flex flex-wrap items-center gap-1.5",
                    for l in card_labels() {
                        button {
                            key: "{l}",
                            r#type: "button",
                            title: "Entfernen",
                            onclick: {
                                let l = l.clone();
                                move |_| card_labels.write().retain(|x| x != &l)
                            },
                            LabelBadge { name: l.clone(), color: super::board_card::label_color(&labels, &l), class: "hover:line-through" }
                        }
                    }
                    input {
                        class: "h-6 w-32 rounded-full border border-dashed border-input bg-transparent px-2 text-[11px] outline-none placeholder:text-muted-foreground focus:border-ring",
                        placeholder: "+ Label",
                        value: new_label(),
                        oninput: move |e| new_label.set(e.value()),
                        onkeydown: move |e| if e.key() == Key::Enter { e.prevent_default(); add_label(); },
                        onblur: move |_| add_label(),
                    }
                }
                if !available.is_empty() {
                    div { class: "mt-1 flex flex-wrap gap-1.5 opacity-60",
                        for l in available {
                            button {
                                key: "{l.id}",
                                r#type: "button",
                                class: "hover:opacity-100",
                                onclick: {
                                    let n = l.name.clone();
                                    move |_| card_labels.write().push(n.clone())
                                },
                                LabelBadge { name: l.name.clone(), color: l.color.clone() }
                            }
                        }
                    }
                }
            }

            Field { label: "Beschreibung",
                textarea {
                    class: TEXTAREA,
                    placeholder: "Details, Notizen, Links …",
                    value: description(),
                    oninput: move |e| description.set(e.value()),
                }
            }

            Field { label: "Link",
                div { class: "flex gap-2",
                    input { class: INPUT, placeholder: "https://…", value: url(), oninput: move |e| url.set(e.value()) }
                    if !url().is_empty() {
                        a { class: "flex items-center rounded-md border px-2 text-muted-foreground hover:text-foreground", href: url(), target: "_blank",
                            ExternalLink { class: "size-4" }
                        }
                    }
                }
            }

            div { class: "flex flex-col gap-2",
                div { class: "flex items-center justify-between",
                    span { class: "text-xs font-medium text-muted-foreground", "Checkliste" }
                    if !detail.checklist.is_empty() {
                        span { class: "text-xs text-muted-foreground", "{done_count}/{detail.checklist.len()}" }
                    }
                }
                if !detail.checklist.is_empty() {
                    div { class: "h-1 overflow-hidden rounded-full bg-muted",
                        div { class: "h-full bg-emerald-500 transition-all", style: "width: {done_count * 100 / detail.checklist.len()}%" }
                    }
                }
                for item in detail.checklist.iter().cloned() {
                    div { key: "{item.id}", class: "group flex items-center gap-2 rounded-md px-1 py-1 hover:bg-accent/40",
                        input {
                            r#type: "checkbox",
                            class: "size-4 accent-neutral-300",
                            checked: item.done,
                            onchange: {
                                let iid = item.id.clone();
                                move |e: FormEvent| {
                                    let iid = iid.clone();
                                    let checked = e.checked();
                                    spawn(async move {
                                        let _ = set_checklist_item(iid, checked).await;
                                        on_changed.call(());
                                        detail_res.restart();
                                    });
                                }
                            },
                        }
                        span { class: if item.done { "flex-1 text-sm text-muted-foreground line-through" } else { "flex-1 text-sm" }, "{item.text}" }
                        button {
                            class: "opacity-0 text-muted-foreground group-hover:opacity-100 hover:text-destructive",
                            onclick: {
                                let iid = item.id.clone();
                                move |_| {
                                    let iid = iid.clone();
                                    spawn(async move {
                                        let _ = delete_checklist_item(iid).await;
                                        on_changed.call(());
                                        detail_res.restart();
                                    });
                                }
                            },
                            X { class: "size-3.5" }
                        }
                    }
                }
                form { class: "flex gap-2", onsubmit: add_item,
                    input { class: INPUT, placeholder: "Punkt hinzufügen …", value: new_item(), oninput: move |e| new_item.set(e.value()) }
                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Icon, submit: true, Plus { class: "size-4" } }
                }
            }

            div { class: "flex flex-wrap items-center gap-x-4 gap-y-1 rounded-lg bg-muted/50 px-3 py-2 text-[11px] text-muted-foreground",
                if let Some(s) = &card.source {
                    span { class: "inline-flex items-center gap-1",
                        Zap { class: "size-3" }
                        "Quelle: {s}"
                        if let Some(x) = &card.external_id { " · #{x}" }
                    }
                }
                span { "Erstellt {created}" }
                button {
                    class: "inline-flex items-center gap-1 font-mono hover:text-foreground",
                    title: "API-Pfad kopieren",
                    onclick: {
                        let p = api_path.clone();
                        move |_| copy_to_clipboard(p.clone())
                    },
                    Copy { class: "size-3" }
                    "{api_path}"
                }
            }
        }
        div { class: "sticky bottom-0 flex items-center gap-2 border-t bg-popover px-5 py-3",
            Button {
                variant: ButtonVariant::Ghost,
                class: "text-muted-foreground hover:text-destructive",
                onclick: move |_| {
                    let id = id_del.clone();
                    spawn(async move {
                        if delete_card(id).await.is_ok() {
                            onclose.call(true);
                        }
                    });
                },
                Trash2 { class: "size-4" }
                "Löschen"
            }
            div { class: "flex-1" }
            Button { variant: ButtonVariant::Ghost, onclick: move |_| onclose.call(false), "Schliessen" }
            Button { onclick: save, "Speichern" }
        }
    }
}
