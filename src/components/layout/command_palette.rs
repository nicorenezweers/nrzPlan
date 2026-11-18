use dioxus::prelude::*;
use icons::{CalendarDays, CircleCheck, ListTodo, Plus, Search, Settings, SquareKanban, Sun};

use crate::api::{create_todo, search};
use crate::app::Route;
use crate::model::{NewTodo, SearchResults};
use crate::util::{fmt_relative, fmt_short, fmt_time, parse_quick_add, recurrence_hint, today};

/// Globaler Zustand: Palette offen?
#[derive(Clone, Copy)]
pub struct Palette(pub Signal<bool>);

/// Wird hochgezählt, wenn ausserhalb einer Seite Daten geändert wurden (z.B. Todo aus der Palette).
#[derive(Clone, Copy)]
pub struct Refresh(pub Signal<u64>);

#[derive(Clone, PartialEq)]
enum Action {
    Go(Route),
    CreateTodo(String),
}

#[derive(Clone, PartialEq)]
struct Item {
    group: &'static str,
    icon: &'static str,
    title: String,
    hint: String,
    action: Action,
}

fn nav_items() -> Vec<Item> {
    [
        ("Heute", "sun", Route::Today {}),
        ("Todos", "todo", Route::Todos {}),
        ("Kalender", "cal", Route::Calendar {}),
        ("Boards", "board", Route::Boards {}),
        ("Einstellungen", "settings", Route::Settings {}),
    ]
    .into_iter()
    .map(|(t, i, r)| Item { group: "Navigation", icon: i, title: t.into(), hint: String::new(), action: Action::Go(r) })
    .collect()
}

fn result_items(r: &SearchResults) -> Vec<Item> {
    let mut v = Vec::new();
    for b in &r.boards {
        v.push(Item {
            group: "Boards",
            icon: "board",
            title: b.name.clone(),
            hint: "Board".into(),
            action: Action::Go(Route::Board { id: b.id.clone() }),
        });
    }
    for c in &r.cards {
        v.push(Item {
            group: "Karten",
            icon: "board",
            title: c.card.title.clone(),
            hint: format!("{} · {}", c.board_name, c.column_name),
            action: Action::Go(Route::Board { id: c.card.board_id.clone() }),
        });
    }
    for t in &r.todos {
        v.push(Item {
            group: "Todos",
            icon: if t.done { "done" } else { "todo" },
            title: t.title.clone(),
            hint: t.due_date.map(fmt_relative).unwrap_or_default(),
            action: Action::Go(Route::Todos {}),
        });
    }
    for e in &r.events {
        let when = if e.all_day {
            fmt_short(e.start_at.date())
        } else {
            format!("{} {}", fmt_short(e.start_at.date()), fmt_time(e.start_at))
        };
        v.push(Item { group: "Termine", icon: "cal", title: e.title.clone(), hint: when, action: Action::Go(Route::Calendar {}) });
    }
    v
}

#[component]
fn ItemIcon(name: &'static str) -> Element {
    let c = "size-4 shrink-0 text-muted-foreground";
    match name {
        "sun" => rsx! { Sun { class: c } },
        "todo" => rsx! { ListTodo { class: c } },
        "done" => rsx! { CircleCheck { class: c } },
        "cal" => rsx! { CalendarDays { class: c } },
        "board" => rsx! { SquareKanban { class: c } },
        "settings" => rsx! { Settings { class: c } },
        _ => rsx! { Plus { class: c } },
    }
}

/// ⌘K / Strg+K: suchen, navigieren, Todo anlegen.
#[component]
pub fn CommandPalette() -> Element {
    let mut open = use_context::<Palette>().0;

    // Globaler Shortcut (nur im Browser; auf dem Server schlägt eval fehl).
    use_future(move || async move {
        let mut ev = document::eval(
            r#"document.addEventListener('keydown', (e) => {
                if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); dioxus.send('k'); }
            });
            await new Promise(() => {});"#,
        );
        while ev.recv::<String>().await.is_ok() {
            open.toggle();
        }
    });

    if !open() {
        return rsx! {};
    }
    rsx! { PaletteDialog {} }
}

#[component]
fn PaletteDialog() -> Element {
    let mut open = use_context::<Palette>().0;
    let mut refresh = use_context::<Refresh>().0;
    let navigator = use_navigator();
    let mut query = use_signal(String::new);
    let mut selected = use_signal(|| 0usize);
    let mut flash = use_signal(|| None::<String>);

    let results = use_resource(move || async move {
        let q = query();
        if q.trim().chars().count() < 2 {
            return SearchResults::default();
        }
        search(q).await.unwrap_or_default()
    });

    let q = query();
    let ql = q.trim().to_lowercase();
    let mut items: Vec<Item> = Vec::new();
    if !ql.is_empty() {
        let parsed = parse_quick_add(&q, today());
        let mut hint = Vec::new();
        if let Some(d) = parsed.due {
            hint.push(fmt_relative(d));
        }
        if parsed.priority > 0 {
            hint.push("!".repeat(parsed.priority as usize));
        }
        if let Some(l) = &parsed.list {
            hint.push(format!("#{l}"));
        }
        if let Some(r) = &parsed.recurrence {
            hint.push(recurrence_hint(r));
        }
        items.push(Item {
            group: "Anlegen",
            icon: "plus",
            title: format!("Todo „{}“", if parsed.title.is_empty() { q.trim() } else { &parsed.title }),
            hint: hint.join(" · "),
            action: Action::CreateTodo(q.clone()),
        });
    }
    items.extend(nav_items().into_iter().filter(|i| ql.is_empty() || i.title.to_lowercase().contains(&ql)));
    if let Some(r) = &*results.read() {
        items.extend(result_items(r));
    }
    let count = items.len();
    let sel = selected().min(count.saturating_sub(1));

    let mut run = move |item: Item| match item.action {
        Action::Go(route) => {
            open.set(false);
            navigator.push(route);
        }
        Action::CreateTodo(text) => {
            let p = parse_quick_add(&text, today());
            let title = if p.title.trim().is_empty() { text.trim().to_string() } else { p.title };
            let input =
                NewTodo { title, due_date: p.due, priority: Some(p.priority), list: p.list, recurrence: p.recurrence, ..Default::default() };
            spawn(async move {
                match create_todo(input).await {
                    Ok(t) => {
                        refresh += 1;
                        query.set(String::new());
                        flash.set(Some(format!("✓ „{}“ angelegt", t.title)));
                    }
                    Err(e) => flash.set(Some(format!("Fehler: {e}"))),
                }
            });
        }
    };

    let items_for_key = items.clone();
    let mut last_group = "";

    rsx! {
        div {
            class: "fixed inset-0 z-[300] flex items-start justify-center bg-black/60 px-3 pt-[12vh] backdrop-blur-[2px]",
            onclick: move |_| open.set(false),
            div {
                class: "w-full max-w-xl overflow-hidden rounded-xl border bg-popover text-popover-foreground shadow-2xl",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex items-center gap-3 border-b px-4",
                    Search { class: "size-4 text-muted-foreground" }
                    input {
                        class: "h-12 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground",
                        placeholder: "Suchen oder Todo anlegen … (z.B. „Zahnarzt anrufen morgen !!“)",
                        value: query(),
                        onmounted: move |e| async move { let _ = e.set_focus(true).await; },
                        oninput: move |e| {
                            query.set(e.value());
                            selected.set(0);
                            flash.set(None);
                        },
                        onkeydown: move |e| match e.key() {
                            Key::Escape => open.set(false),
                            Key::ArrowDown => {
                                e.prevent_default();
                                if count > 0 { selected.set((sel + 1) % count); }
                            }
                            Key::ArrowUp => {
                                e.prevent_default();
                                if count > 0 { selected.set((sel + count - 1) % count); }
                            }
                            Key::Enter => {
                                e.prevent_default();
                                if let Some(item) = items_for_key.get(sel).cloned() { run(item); }
                            }
                            _ => {}
                        },
                    }
                    kbd { class: "rounded border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground", "esc" }
                }
                if let Some(f) = flash() {
                    div { class: "border-b bg-emerald-500/10 px-4 py-2 text-xs text-emerald-300", "{f}" }
                }
                div { class: "max-h-[50vh] overflow-y-auto p-1.5",
                    if items.is_empty() {
                        p { class: "px-3 py-6 text-center text-sm text-muted-foreground", "Nichts gefunden." }
                    }
                    for (i, item) in items.into_iter().enumerate() {
                        {
                            let header = (item.group != last_group).then_some(item.group);
                            last_group = item.group;
                            let it = item.clone();
                            rsx! {
                                if let Some(h) = header {
                                    p { class: "px-3 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground", "{h}" }
                                }
                                button {
                                    class: if i == sel { "flex w-full items-center gap-3 rounded-lg bg-accent px-3 py-2 text-left text-sm" } else { "flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm hover:bg-accent/50" },
                                    onmouseenter: move |_| selected.set(i),
                                    onclick: move |_| run(it.clone()),
                                    ItemIcon { name: item.icon }
                                    span { class: "min-w-0 flex-1 truncate", "{item.title}" }
                                    if !item.hint.is_empty() {
                                        span { class: "shrink-0 truncate text-xs text-muted-foreground", "{item.hint}" }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "flex items-center gap-4 border-t px-4 py-2 text-[11px] text-muted-foreground",
                    span { "↑↓ auswählen" }
                    span { "↵ öffnen" }
                    span { class: "ml-auto", "⌘K / Strg+K" }
                }
            }
        }
    }
}
