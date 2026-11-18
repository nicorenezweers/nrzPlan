use chrono::{Duration, NaiveDate};
use dioxus::prelude::*;
use icons::{ChevronDown, ChevronRight, Plus};

use crate::api::{create_todo, get_todos};
use crate::components::layout::page::{PageHeader, Section};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::skeleton::Skeleton;
use crate::domain::todos::todo_row::TodoRow;
use crate::model::{NewTodo, Todo, recurrence_label};
use crate::util::{fmt_relative, parse_quick_add, today};

/// Schnell-Eingabe für Todos mit Erkennung von Datum, Priorität und Liste.
#[component]
pub fn QuickAddTodo(
    on_added: EventHandler<()>,
    #[props(into, optional)] placeholder: Option<String>,
    /// Ohne Datumsangabe dieses Datum setzen (z.B. heute).
    #[props(default)]
    default_date: Option<NaiveDate>,
    #[props(into, optional)] default_list: Option<String>,
) -> Element {
    let mut text = use_signal(String::new);
    let preview = use_memo(move || parse_quick_add(&text(), today()));

    let submit = move |e: FormEvent| {
        e.prevent_default();
        let q = parse_quick_add(&text(), today());
        if q.title.trim().is_empty() {
            return;
        }
        let input = NewTodo {
            title: q.title,
            due_date: q.due.or(default_date),
            priority: Some(q.priority),
            list: q.list.or(default_list.clone()),
            recurrence: q.recurrence,
            ..Default::default()
        };
        spawn(async move {
            if create_todo(input).await.is_ok() {
                text.set(String::new());
                on_added.call(());
            }
        });
    };

    rsx! {
        form { class: "flex flex-col gap-1", onsubmit: submit,
            div { class: "flex items-center gap-3 rounded-lg border border-dashed border-input px-3 py-2 focus-within:border-solid focus-within:border-ring",
                Plus { class: "size-4 text-muted-foreground" }
                input {
                    class: "flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground",
                    placeholder: placeholder.unwrap_or_else(|| "Neues Todo … (morgen, freitag, 24.12., !!, #liste, täglich)".into()),
                    value: text(),
                    oninput: move |e| text.set(e.value()),
                }
            }
            if !text().is_empty() && (preview().due.is_some() || preview().priority > 0 || preview().list.is_some() || preview().recurrence.is_some()) {
                div { class: "flex gap-2 px-3 text-[11px] text-muted-foreground",
                    if let Some(d) = preview().due { span { "📅 {fmt_relative(d)}" } }
                    if preview().priority > 0 { span { "⚑ {\"!\".repeat(preview().priority as usize)}" } }
                    if let Some(l) = preview().list { span { "# {l}" } }
                    if let Some(r) = preview().recurrence { span { "↻ {recurrence_label(&r)}" } }
                }
            }
        }
    }
}

#[component]
pub fn TodosPage() -> Element {
    let refresh = use_context::<crate::components::layout::command_palette::Refresh>().0;
    let mut todos = use_server_future(move || {
        let _ = refresh();
        get_todos()
    })?;
    let mut filter = use_signal(|| None::<String>);
    let mut show_done = use_signal(|| false);
    let reload = move |_| todos.restart();

    let data: Vec<Todo> = match &*todos.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let loading = todos.read().is_none();
    let error = match &*todos.read() {
        Some(Err(e)) => Some(e.to_string()),
        _ => None,
    };

    let mut lists: Vec<String> = data.iter().filter_map(|t| t.list.clone()).collect();
    lists.sort_by_key(|l| l.to_lowercase());
    lists.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    let visible: Vec<Todo> = data
        .into_iter()
        .filter(|t| filter().is_none_or(|f| t.list.as_deref().is_some_and(|l| l.eq_ignore_ascii_case(&f))))
        .collect();
    let t0 = today();
    let open = |pred: &dyn Fn(&Todo) -> bool| -> Vec<Todo> {
        visible.iter().filter(|t| !t.done && pred(t)).cloned().collect()
    };
    let overdue = open(&|t| t.due_date.is_some_and(|d| d < t0));
    let today_list = open(&|t| t.due_date == Some(t0));
    let week = open(&|t| t.due_date.is_some_and(|d| d > t0 && d <= t0 + Duration::days(7)));
    let later = open(&|t| t.due_date.is_some_and(|d| d > t0 + Duration::days(7)));
    let someday = open(&|t| t.due_date.is_none());
    let done: Vec<Todo> = visible.iter().filter(|t| t.done).cloned().collect();
    let open_count = visible.len() - done.len();

    let chip = |active: bool| {
        if active {
            "rounded-full border border-foreground/30 bg-accent px-3 py-1 text-xs font-medium"
        } else {
            "rounded-full border px-3 py-1 text-xs text-muted-foreground hover:text-foreground"
        }
    };

    rsx! {
        div { class: "mx-auto flex w-full max-w-3xl flex-col",
            PageHeader { title: "Todos", subtitle: format!("{open_count} offen") }
            div { class: "flex flex-col gap-6 px-4 pb-10 sm:px-8",
                QuickAddTodo { on_added: reload, default_list: filter() }

                if !lists.is_empty() {
                    div { class: "flex flex-wrap gap-2",
                        button { class: chip(filter().is_none()), onclick: move |_| filter.set(None), "Alle" }
                        for l in lists {
                            button {
                                key: "{l}",
                                class: chip(filter().as_deref() == Some(l.as_str())),
                                onclick: {
                                    let l = l.clone();
                                    move |_| filter.set(Some(l.clone()))
                                },
                                "# {l}"
                            }
                        }
                    }
                }

                if loading {
                    div { class: "flex flex-col gap-2",
                        Skeleton { class: "h-10 w-full" }
                        Skeleton { class: "h-10 w-full" }
                        Skeleton { class: "h-10 w-full" }
                    }
                } else if let Some(err) = error {
                    p { class: "text-sm text-destructive", "Fehler: {err}" }
                } else if open_count == 0 && done.is_empty() {
                    Empty {
                        EmptyTitle { "Alles erledigt ✨" }
                        EmptyDescription { "Lege oben dein erstes Todo an." }
                    }
                } else {
                    TodoGroup { title: "Überfällig", todos: overdue, on_change: reload, danger: true }
                    TodoGroup { title: "Heute", todos: today_list, on_change: reload, hide_date: true }
                    TodoGroup { title: "Nächste 7 Tage", todos: week, on_change: reload }
                    TodoGroup { title: "Später", todos: later, on_change: reload }
                    TodoGroup { title: "Irgendwann", todos: someday, on_change: reload }
                    if !done.is_empty() {
                        div { class: "flex flex-col gap-1",
                            button {
                                class: "flex items-center gap-1 px-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground hover:text-foreground",
                                onclick: move |_| show_done.toggle(),
                                if show_done() { ChevronDown { class: "size-3.5" } } else { ChevronRight { class: "size-3.5" } }
                                "Erledigt ({done.len()})"
                            }
                            if show_done() {
                                for t in done {
                                    TodoRow { key: "{t.id}", todo: t.clone(), on_change: reload }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TodoGroup(
    #[props(into)] title: String,
    todos: Vec<Todo>,
    on_change: EventHandler<()>,
    #[props(default)] danger: bool,
    #[props(default)] hide_date: bool,
) -> Element {
    if todos.is_empty() {
        return rsx! {};
    }
    rsx! {
        Section { title, count: todos.len(), title_class: if danger { "text-red-400" } else { "" },
            div { class: "flex flex-col",
                for t in todos {
                    TodoRow { key: "{t.id}", todo: t.clone(), on_change, hide_date }
                }
            }
        }
    }
}
