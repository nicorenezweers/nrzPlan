use dioxus::prelude::*;
use icons::{CalendarDays, CircleCheck, Flag, ListTodo, Search};

use crate::api::get_today;
use crate::app::Route;
use crate::components::layout::page::Section;
use crate::components::ui::skeleton::Skeleton;
use crate::domain::calendar::event_dialog::EventDialog;
use crate::domain::calendar::page_calendar::{CardItem, EventItem};
use crate::domain::todos::page_todos::QuickAddTodo;
use crate::domain::today::focus_timer::FocusTimer;
use crate::domain::todos::todo_row::TodoRow;
use crate::model::Event;
use crate::util::{fmt_long, greeting, today};

#[component]
fn Stat(#[props(into)] label: String, value: i64, #[props(into, optional)] tone: Option<String>, children: Element) -> Element {
    let tone = tone.unwrap_or_default();
    rsx! {
        div { class: "flex items-center gap-3 rounded-xl border bg-card/40 px-4 py-3",
            div { class: "flex size-9 items-center justify-center rounded-lg bg-muted {tone}", {children} }
            div {
                p { class: "text-xl font-semibold leading-none tabular-nums", "{value}" }
                p { class: "mt-1 text-xs text-muted-foreground", "{label}" }
            }
        }
    }
}

#[component]
pub fn TodayPage() -> Element {
    let date = today();
    let refresh = use_context::<crate::components::layout::command_palette::Refresh>().0;
    let mut data = use_server_future(move || {
        let _ = refresh();
        get_today(date)
    })?;
    let reload = move |_| data.restart();
    let mut edit_event = use_signal(|| None::<Event>);
    let mut palette = use_context::<crate::components::layout::command_palette::Palette>().0;

    let body = match &*data.read() {
        Some(Ok(d)) => {
            let d = d.clone();
            let focus_count = d.overdue.len() + d.today.len();
            rsx! {
                div { class: "grid grid-cols-2 gap-3 lg:grid-cols-4",
                    Stat { label: "Termine heute", value: d.events.len() as i64, CalendarDays { class: "size-4" } }
                    Stat { label: "Todos heute", value: d.today.len() as i64, ListTodo { class: "size-4" } }
                    Stat { label: "Überfällig", value: d.overdue.len() as i64, tone: if d.overdue.is_empty() { "" } else { "text-red-400" }, Flag { class: "size-4" } }
                    Stat { label: "Heute erledigt", value: d.done_today, tone: "text-emerald-400", CircleCheck { class: "size-4" } }
                }
                div { class: "grid gap-6 lg:grid-cols-[1fr_20rem]",
                    div { class: "flex flex-col gap-6",
                        Section { title: "Fokus für heute", count: focus_count,
                            for t in d.overdue.iter().cloned() {
                                TodoRow { key: "{t.id}", todo: t, on_change: reload }
                            }
                            for t in d.today.iter().cloned() {
                                TodoRow { key: "{t.id}", todo: t, on_change: reload, hide_date: true }
                            }
                            if focus_count == 0 {
                                p { class: "px-2 py-1 text-sm text-muted-foreground", "Nichts fällig – Zeit für Deep Work. ✨" }
                            }
                            div { class: "mt-1",
                                QuickAddTodo { on_added: reload, default_date: Some(date), placeholder: "Was steht heute an? (!!, #liste)" }
                            }
                        }
                        if !d.upcoming.is_empty() {
                            Section { title: "Nächste 7 Tage", count: d.upcoming.len(),
                                for t in d.upcoming.iter().cloned() {
                                    TodoRow { key: "{t.id}", todo: t, on_change: reload }
                                }
                            }
                        }
                        if !d.cards_due.is_empty() {
                            Section { title: "Fällige Board-Karten", count: d.cards_due.len(),
                                for c in d.cards_due.iter().cloned() {
                                    CardItem { key: "{c.card.id}", card: c }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-6",
                        Section { title: "Termine", count: d.events.len(),
                            if d.events.is_empty() {
                                p { class: "px-1 text-sm text-muted-foreground", "Keine Termine heute." }
                            }
                            for e in d.events.iter().cloned() {
                                EventItem { key: "{e.id}", event: e, day: date, onclick: move |e: Event| edit_event.set(Some(e)) }
                            }
                            Link { to: Route::Calendar {}, class: "px-1 pt-1 text-xs text-muted-foreground hover:text-foreground", "Zum Kalender →" }
                        }
                        FocusTimer {}
                        if d.open_total > 0 {
                            p { class: "px-1 text-xs text-muted-foreground", "Insgesamt {d.open_total} offene Todos." }
                        }
                    }
                }
            }
        }
        Some(Err(e)) => rsx! { p { class: "text-sm text-destructive", "Fehler: {e}" } },
        None => rsx! {
            div { class: "grid grid-cols-2 gap-3 lg:grid-cols-4",
                Skeleton { class: "h-16" }
                Skeleton { class: "h-16" }
                Skeleton { class: "h-16" }
                Skeleton { class: "h-16" }
            }
        },
    };

    rsx! {
        div { class: "mx-auto flex w-full max-w-6xl flex-col gap-6 px-4 pb-10 pt-[calc(env(safe-area-inset-top)+1.25rem)] sm:px-8 sm:pt-7",
            div { class: "flex items-start gap-3",
                div { class: "flex-1",
                    h1 { class: "text-2xl font-semibold tracking-tight", "{greeting()}" }
                    p { class: "text-sm text-muted-foreground", "{fmt_long(date)}" }
                }
                button {
                    class: "flex h-9 items-center gap-2 rounded-lg border px-3 text-sm text-muted-foreground hover:text-foreground",
                    onclick: move |_| palette.set(true),
                    Search { class: "size-4" }
                    span { class: "hidden sm:inline", "Suchen" }
                    kbd { class: "hidden rounded border px-1 font-mono text-[10px] sm:inline", "⌘K" }
                }
            }
            {body}
        }
        if let Some(e) = edit_event() {
            EventDialog {
                key: "{e.id}",
                date: e.start_at.date(),
                event: Some(e.clone()),
                onclose: move |r: bool| {
                    edit_event.set(None);
                    if r {
                        data.restart();
                    }
                },
            }
        }
    }
}
