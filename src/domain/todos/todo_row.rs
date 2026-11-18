use dioxus::prelude::*;
use icons::{Flag, Hash, Repeat, Trash2, Zap};

use crate::api::{delete_todo, update_todo};
use crate::components::ui::badge::Pill;
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::check::TodoCheck;
use crate::components::ui::field::{Field, INPUT, SELECT, TEXTAREA};
use crate::components::ui::modal::Modal;
use crate::model::{RECURRENCES, Todo, TodoPatch, recurrence_label};
use crate::util::{date_input, fmt_relative, parse_date, priority_class, priority_label, today};

/// Eine Todo-Zeile mit Abhaken; Klick öffnet den Bearbeiten-Dialog.
#[component]
pub fn TodoRow(todo: Todo, on_change: EventHandler<()>, #[props(default)] hide_date: bool) -> Element {
    let mut editing = use_signal(|| false);
    let mut done = use_signal(|| todo.done);
    let id = todo.id.clone();
    let overdue = !todo.done && todo.due_date.is_some_and(|d| d < today());

    rsx! {
        div {
            class: "group flex cursor-pointer items-start gap-3 rounded-lg px-2 py-2 transition-colors hover:bg-accent/40",
            onclick: move |_| editing.set(true),
            div { class: "pt-0.5",
                TodoCheck {
                    checked: done(),
                    priority: todo.priority,
                    onclick: move |_| {
                        let id = id.clone();
                        let next = !done();
                        done.set(next);
                        spawn(async move {
                            let _ = update_todo(id, TodoPatch { done: Some(next), ..Default::default() }).await;
                            on_change.call(());
                        });
                    },
                }
            }
            div { class: "flex min-w-0 flex-1 flex-col gap-1",
                span { class: if done() { "text-sm text-muted-foreground line-through" } else { "text-sm" }, "{todo.title}" }
                if let Some(n) = &todo.notes {
                    span { class: "line-clamp-1 text-xs text-muted-foreground", "{n}" }
                }
                div { class: "flex flex-wrap items-center gap-1.5 empty:hidden",
                    if let Some(d) = todo.due_date.filter(|_| !hide_date) {
                        Pill { class: if overdue { "text-red-400 bg-red-500/10" } else { "" }, "{fmt_relative(d)}" }
                    }
                    if let Some(l) = &todo.list {
                        Pill { Hash { class: "size-3" } "{l}" }
                    }
                    if let Some(r) = &todo.recurrence {
                        Pill { Repeat { class: "size-3" } "{recurrence_label(r)}" }
                    }
                    if todo.priority > 0 {
                        Pill { Flag { class: "size-3 {priority_class(todo.priority)}" } "{priority_label(todo.priority)}" }
                    }
                    if let Some(s) = &todo.source {
                        Pill { Zap { class: "size-3" } "{s}" }
                    }
                }
            }
        }
        if editing() {
            TodoDialog {
                todo: todo.clone(),
                onclose: move |changed: bool| {
                    editing.set(false);
                    if changed {
                        on_change.call(());
                    }
                },
            }
        }
    }
}

#[component]
pub fn TodoDialog(todo: Todo, onclose: EventHandler<bool>) -> Element {
    let mut title = use_signal(|| todo.title.clone());
    let mut notes = use_signal(|| todo.notes.clone().unwrap_or_default());
    let mut list = use_signal(|| todo.list.clone().unwrap_or_default());
    let mut due = use_signal(|| date_input(todo.due_date));
    let mut priority = use_signal(|| todo.priority);
    let mut recurrence = use_signal(|| todo.recurrence.clone().unwrap_or_default());
    let id = todo.id.clone();
    let id_del = todo.id.clone();

    let save = move |_| {
        let id = id.clone();
        let patch = TodoPatch {
            title: Some(title()),
            notes: Some(Some(notes())),
            list: Some(Some(list())),
            due_date: Some(parse_date(&due())),
            priority: Some(priority()),
            done: None,
            recurrence: Some(Some(recurrence()).filter(|r| !r.is_empty())),
        };
        spawn(async move {
            if update_todo(id, patch).await.is_ok() {
                onclose.call(true);
            }
        });
    };

    rsx! {
        Modal { title: "Todo bearbeiten", onclose: move |_| onclose.call(false),
            div { class: "flex flex-col gap-4 p-5",
                Field { label: "Titel",
                    input { class: INPUT, value: title(), oninput: move |e| title.set(e.value()) }
                }
                Field { label: "Notizen",
                    textarea { class: TEXTAREA, value: notes(), oninput: move |e| notes.set(e.value()) }
                }
                div { class: "grid grid-cols-2 gap-3",
                    Field { label: "Fällig",
                        input { r#type: "date", class: INPUT, value: due(), oninput: move |e| due.set(e.value()) }
                    }
                    Field { label: "Priorität",
                        select {
                            class: SELECT,
                            value: "{priority()}",
                            onchange: move |e| priority.set(e.value().parse().unwrap_or(0)),
                            for p in [0i64, 1, 2, 3] {
                                option { value: "{p}", selected: p == priority(), "{priority_label(p)}" }
                            }
                        }
                    }
                }
                div { class: "grid grid-cols-2 gap-3",
                    Field { label: "Liste",
                        input { class: INPUT, placeholder: "z.B. Arbeit, Privat", value: list(), oninput: move |e| list.set(e.value()) }
                    }
                    Field { label: "Wiederholen",
                        select { class: SELECT, onchange: move |e| recurrence.set(e.value()),
                            option { value: "", selected: recurrence().is_empty(), "Nie" }
                            for (k, l) in RECURRENCES {
                                option { value: k, selected: recurrence() == k, "{l}" }
                            }
                        }
                    }
                }
                if !recurrence().is_empty() {
                    p { class: "-mt-2 text-xs text-muted-foreground", "Beim Abhaken wird automatisch die nächste Instanz angelegt." }
                }
                if let Some(s) = &todo.source {
                    p { class: "text-xs text-muted-foreground",
                        "Extern angelegt über "
                        span { class: "font-medium text-foreground", "{s}" }
                        if let Some(x) = &todo.external_id { " (#{x})" }
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
                            if delete_todo(id).await.is_ok() {
                                onclose.call(true);
                            }
                        });
                    },
                    Trash2 { class: "size-4" }
                    "Löschen"
                }
                div { class: "flex-1" }
                Button { variant: ButtonVariant::Ghost, onclick: move |_| onclose.call(false), "Abbrechen" }
                Button { onclick: save, "Speichern" }
            }
        }
    }
}
