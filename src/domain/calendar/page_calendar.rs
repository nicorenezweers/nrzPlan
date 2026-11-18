use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;
use icons::{ChevronLeft, ChevronRight, Clock, MapPin, Plus, SquareKanban};

use crate::api::get_calendar;
use crate::app::Route;
use crate::components::layout::page::{PageHeader, Section};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::domain::calendar::event_dialog::EventDialog;
use crate::domain::todos::page_todos::QuickAddTodo;
use crate::domain::todos::todo_row::TodoRow;
use crate::model::{CalendarData, CardRef, Event, Todo};
use crate::util::{
    MONTHS, WEEKDAYS_SHORT, add_months, dot_class, event_class, fmt_long, fmt_time, month_grid, today, week_start,
};

#[derive(Clone, Copy, PartialEq)]
enum View {
    Month,
    Week,
}

fn on_day(e: &Event, d: NaiveDate) -> bool {
    e.start_at.date() <= d && e.end_at.date() >= d
}

fn event_time(e: &Event, d: NaiveDate) -> String {
    if e.all_day {
        "Ganztägig".into()
    } else if e.start_at.date() == d && e.end_at.date() == d {
        format!("{} – {}", fmt_time(e.start_at), fmt_time(e.end_at))
    } else if e.start_at.date() == d {
        format!("ab {}", fmt_time(e.start_at))
    } else if e.end_at.date() == d {
        format!("bis {}", fmt_time(e.end_at))
    } else {
        "Ganztägig".into()
    }
}

#[component]
pub fn CalendarPage() -> Element {
    let t0 = today();
    let mut cursor = use_signal(|| (t0.year(), t0.month()));
    let mut week_anchor = use_signal(|| week_start(t0));
    let mut view = use_signal(|| View::Month);
    let mut selected = use_signal(|| t0);
    let mut new_event = use_signal(|| None::<NaiveDate>);
    let mut edit_event = use_signal(|| None::<Event>);

    let range = use_memo(move || match view() {
        View::Month => {
            let (y, m) = cursor();
            let grid = month_grid(y, m);
            (grid[0], grid[41])
        }
        View::Week => (week_anchor(), week_anchor() + Duration::days(6)),
    });
    let mut data = use_server_future(move || {
        let (from, to) = range();
        get_calendar(from, to)
    })?;
    let reload = move |_| data.restart();

    let cal: CalendarData = match &*data.read() {
        Some(Ok(d)) => d.clone(),
        _ => CalendarData { events: vec![], todos: vec![], cards: vec![] },
    };

    let title = match view() {
        View::Month => {
            let (y, m) = cursor();
            format!("{} {}", MONTHS[m as usize - 1], y)
        }
        View::Week => {
            let s = week_anchor();
            let e = s + Duration::days(6);
            format!("KW {} · {}.{}. – {}.{}.{}", s.iso_week().week(), s.day(), s.month(), e.day(), e.month(), e.year())
        }
    };

    let mut go = move |delta: i32| match view() {
        View::Month => {
            let (y, m) = cursor();
            cursor.set(add_months(y, m, delta));
        }
        View::Week => week_anchor.set(week_anchor() + Duration::days(7 * delta as i64)),
    };
    let go_today = move |_| {
        let t = today();
        cursor.set((t.year(), t.month()));
        week_anchor.set(week_start(t));
        selected.set(t);
    };

    let sel = selected();
    let day_events: Vec<Event> = cal.events.iter().filter(|e| on_day(e, sel)).cloned().collect();
    let day_todos: Vec<Todo> = cal.todos.iter().filter(|t| t.due_date == Some(sel)).cloned().collect();
    let day_cards: Vec<CardRef> = cal.cards.iter().filter(|c| c.card.due_date == Some(sel)).cloned().collect();

    let seg = |active: bool| {
        if active { "rounded-md bg-accent px-3 py-1 text-xs font-medium" } else { "rounded-md px-3 py-1 text-xs text-muted-foreground hover:text-foreground" }
    };

    rsx! {
        div { class: "flex w-full flex-col",
            PageHeader {
                title,
                actions: rsx! {
                    div { class: "flex rounded-lg border p-0.5",
                        button { class: seg(view() == View::Month), onclick: move |_| view.set(View::Month), "Monat" }
                        button {
                            class: seg(view() == View::Week),
                            onclick: move |_| {
                                week_anchor.set(week_start(selected()));
                                view.set(View::Week);
                            },
                            "Woche"
                        }
                    }
                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Sm, onclick: go_today, "Heute" }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, onclick: move |_| go(-1), ChevronLeft { class: "size-4" } }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, onclick: move |_| go(1), ChevronRight { class: "size-4" } }
                    Button { onclick: move |_| new_event.set(Some(selected())),
                        Plus { class: "size-4" }
                        "Termin"
                    }
                },
            }

            div { class: "flex flex-col gap-6 px-4 pb-10 sm:px-8 xl:flex-row",
                div { class: "min-w-0 flex-1",
                    match view() {
                        View::Month => rsx! {
                            MonthGrid {
                                cursor: cursor(),
                                selected: sel,
                                data: cal.clone(),
                                onselect: move |d: NaiveDate| selected.set(d),
                                onadd: move |d: NaiveDate| new_event.set(Some(d)),
                                onevent: move |e: Event| edit_event.set(Some(e)),
                            }
                        },
                        View::Week => rsx! {
                            WeekList {
                                start: week_anchor(),
                                selected: sel,
                                data: cal.clone(),
                                onselect: move |d: NaiveDate| selected.set(d),
                                onevent: move |e: Event| edit_event.set(Some(e)),
                            }
                        },
                    }
                }

                // Tagesdetail
                aside { class: "flex w-full shrink-0 flex-col gap-5 xl:w-80",
                    div {
                        p { class: "text-xs uppercase tracking-wide text-muted-foreground", if sel == today() { "Heute" } else { "Ausgewählt" } }
                        h2 { class: "text-lg font-semibold", "{fmt_long(sel)}" }
                    }
                    Section { title: "Termine", count: day_events.len(),
                        if day_events.is_empty() {
                            p { class: "px-1 text-sm text-muted-foreground", "Keine Termine" }
                        }
                        for e in day_events {
                            EventItem { key: "{e.id}", event: e.clone(), day: sel, onclick: move |e: Event| edit_event.set(Some(e)) }
                        }
                        button {
                            class: "mt-1 flex items-center gap-2 rounded-lg px-2 py-1.5 text-sm text-muted-foreground hover:bg-accent/40 hover:text-foreground",
                            onclick: move |_| new_event.set(Some(selected())),
                            Plus { class: "size-4" }
                            "Termin hinzufügen"
                        }
                    }
                    Section { title: "Todos", count: day_todos.len(),
                        for t in day_todos {
                            TodoRow { key: "{t.id}", todo: t.clone(), on_change: reload, hide_date: true }
                        }
                        QuickAddTodo { on_added: reload, default_date: Some(sel), placeholder: "Todo für diesen Tag …" }
                    }
                    if !day_cards.is_empty() {
                        Section { title: "Board-Karten", count: day_cards.len(),
                            for c in day_cards {
                                CardItem { key: "{c.card.id}", card: c.clone() }
                            }
                        }
                    }
                }
            }
        }

        if let Some(d) = new_event() {
            EventDialog {
                event: None,
                date: d,
                onclose: move |r: bool| {
                    new_event.set(None);
                    if r {
                        data.restart();
                    }
                },
            }
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

#[component]
pub fn EventItem(event: Event, day: NaiveDate, onclick: EventHandler<Event>) -> Element {
    let ev = event.clone();
    rsx! {
        button {
            class: "flex w-full items-start gap-3 rounded-lg border-l-[3px] px-3 py-2 text-left transition-opacity hover:opacity-90 {event_class(&event.color)}",
            onclick: move |_| onclick.call(ev.clone()),
            div { class: "flex min-w-0 flex-1 flex-col gap-0.5",
                span { class: "truncate text-sm font-medium text-foreground", "{event.title}" }
                span { class: "flex items-center gap-1 text-xs opacity-80",
                    Clock { class: "size-3" }
                    "{event_time(&event, day)}"
                }
                if let Some(l) = &event.location {
                    span { class: "flex items-center gap-1 text-xs opacity-80",
                        MapPin { class: "size-3" }
                        "{l}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn CardItem(card: CardRef) -> Element {
    rsx! {
        Link {
            to: Route::Board { id: card.card.board_id.clone() },
            class: "flex items-center gap-3 rounded-lg px-2 py-2 hover:bg-accent/40",
            SquareKanban { class: "size-4 shrink-0 text-muted-foreground" }
            div { class: "min-w-0 flex-1",
                p { class: "truncate text-sm", "{card.card.title}" }
                p { class: "truncate text-xs text-muted-foreground", "{card.board_name} · {card.column_name}" }
            }
        }
    }
}

#[component]
fn MonthGrid(
    cursor: (i32, u32),
    selected: NaiveDate,
    data: CalendarData,
    onselect: EventHandler<NaiveDate>,
    onadd: EventHandler<NaiveDate>,
    onevent: EventHandler<Event>,
) -> Element {
    let (y, m) = cursor;
    let days = month_grid(y, m);
    let t0 = today();
    rsx! {
        div { class: "overflow-hidden rounded-xl border",
            div { class: "grid grid-cols-7 border-b bg-card/40",
                for w in WEEKDAYS_SHORT {
                    div { class: "px-2 py-2 text-center text-xs font-medium text-muted-foreground", "{w}" }
                }
            }
            div { class: "grid grid-cols-7 grid-rows-6",
                for (i, d) in days.into_iter().enumerate() {
                    {
                        let events: Vec<Event> = data.events.iter().filter(|e| on_day(e, d)).cloned().collect();
                        let todos: Vec<&Todo> = data.todos.iter().filter(|t| t.due_date == Some(d)).collect();
                        let cards: Vec<&CardRef> = data.cards.iter().filter(|c| c.card.due_date == Some(d)).collect();
                        let total = events.len() + todos.len() + cards.len();
                        let shown_events: Vec<Event> = events.iter().take(3).cloned().collect();
                        let room = 3usize.saturating_sub(shown_events.len());
                        let shown_todos: Vec<Todo> = todos.iter().take(room).map(|t| (*t).clone()).collect();
                        let room = room.saturating_sub(shown_todos.len());
                        let shown_cards: Vec<CardRef> = cards.iter().take(room).map(|c| (*c).clone()).collect();
                        let more = total - shown_events.len() - shown_todos.len() - shown_cards.len();
                        let in_month = d.month() == m;
                        let is_sel = d == selected;
                        let weekend = d.weekday().num_days_from_monday() >= 5;
                        let cell = format!(
                            "group relative flex min-h-[5.5rem] cursor-pointer flex-col gap-0.5 p-1.5 text-left transition-colors sm:min-h-28 {} {} {} {}",
                            if i % 7 != 6 { "border-r" } else { "" },
                            if i < 35 { "border-b" } else { "" },
                            if is_sel { "bg-accent/50" } else if weekend { "bg-card/20 hover:bg-accent/30" } else { "hover:bg-accent/30" },
                            if in_month { "" } else { "opacity-40" },
                        );
                        rsx! {
                            div {
                                key: "{d}",
                                class: "{cell}",
                                onclick: move |_| onselect.call(d),
                                ondoubleclick: move |_| onadd.call(d),
                                div { class: "flex items-center justify-between",
                                    span {
                                        class: if d == t0 { "flex size-6 items-center justify-center rounded-full bg-primary text-xs font-semibold text-primary-foreground" } else { "flex size-6 items-center justify-center text-xs text-muted-foreground" },
                                        "{d.day()}"
                                    }
                                    button {
                                        class: "hidden rounded p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground sm:group-hover:block",
                                        title: "Termin hinzufügen",
                                        onclick: move |e| {
                                            e.stop_propagation();
                                            onadd.call(d);
                                        },
                                        Plus { class: "size-3.5" }
                                    }
                                }
                                for e in shown_events {
                                    {
                                        let ev = e.clone();
                                        rsx! {
                                            button {
                                                key: "{e.id}",
                                                class: "flex items-center gap-1 truncate rounded px-1 py-px text-left text-[11px] leading-tight hover:bg-accent",
                                                onclick: move |evt| {
                                                    evt.stop_propagation();
                                                    onevent.call(ev.clone());
                                                },
                                                span { class: "size-1.5 shrink-0 rounded-full {dot_class(&e.color)}" }
                                                if !e.all_day && e.start_at.date() == d {
                                                    span { class: "hidden text-muted-foreground md:inline", "{fmt_time(e.start_at)}" }
                                                }
                                                span { class: "truncate", "{e.title}" }
                                            }
                                        }
                                    }
                                }
                                for t in shown_todos {
                                    div { key: "{t.id}", class: "flex items-center gap-1 truncate px-1 text-[11px] leading-tight text-muted-foreground",
                                        span { class: if t.done { "size-2 shrink-0 rounded-full bg-muted-foreground" } else { "size-2 shrink-0 rounded-full border border-muted-foreground" } }
                                        span { class: if t.done { "truncate line-through" } else { "truncate" }, "{t.title}" }
                                    }
                                }
                                for c in shown_cards {
                                    div { key: "{c.card.id}", class: "flex items-center gap-1 truncate px-1 text-[11px] leading-tight text-muted-foreground",
                                        span { class: "size-2 shrink-0 rounded-[2px] border border-muted-foreground" }
                                        span { class: "truncate", "{c.card.title}" }
                                    }
                                }
                                if more > 0 {
                                    span { class: "px-1 text-[10px] text-muted-foreground", "+{more} weitere" }
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
fn WeekList(
    start: NaiveDate,
    selected: NaiveDate,
    data: CalendarData,
    onselect: EventHandler<NaiveDate>,
    onevent: EventHandler<Event>,
) -> Element {
    let t0 = today();
    rsx! {
        div { class: "grid gap-3 md:grid-cols-7",
            for i in 0..7 {
                {
                    let d = start + Duration::days(i);
                    let events: Vec<Event> = data.events.iter().filter(|e| on_day(e, d)).cloned().collect();
                    let todos: Vec<Todo> = data.todos.iter().filter(|t| t.due_date == Some(d)).cloned().collect();
                    let cards: Vec<CardRef> = data.cards.iter().filter(|c| c.card.due_date == Some(d)).cloned().collect();
                    rsx! {
                        div {
                            key: "{d}",
                            class: if d == selected { "flex min-h-40 flex-col gap-1.5 rounded-xl border border-foreground/30 bg-accent/30 p-2" } else { "flex min-h-40 flex-col gap-1.5 rounded-xl border bg-card/30 p-2 hover:bg-accent/20" },
                            onclick: move |_| onselect.call(d),
                            div { class: "flex items-baseline gap-1.5 px-1",
                                span { class: "text-xs text-muted-foreground", "{WEEKDAYS_SHORT[i as usize]}" }
                                span { class: if d == t0 { "rounded-full bg-primary px-1.5 text-sm font-semibold text-primary-foreground" } else { "text-sm font-semibold" }, "{d.day()}" }
                            }
                            for e in events {
                                {
                                    let ev = e.clone();
                                    rsx! {
                                        button {
                                            key: "{e.id}",
                                            class: "flex flex-col rounded-md border-l-2 px-2 py-1 text-left {event_class(&e.color)}",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                onevent.call(ev.clone());
                                            },
                                            span { class: "truncate text-xs font-medium text-foreground", "{e.title}" }
                                            span { class: "text-[10px] opacity-80", "{event_time(&e, d)}" }
                                        }
                                    }
                                }
                            }
                            for t in todos {
                                div { key: "{t.id}", class: "flex items-center gap-1.5 px-1 text-xs text-muted-foreground",
                                    span { class: if t.done { "size-2.5 shrink-0 rounded-full bg-muted-foreground" } else { "size-2.5 shrink-0 rounded-full border border-muted-foreground" } }
                                    span { class: if t.done { "truncate line-through" } else { "truncate" }, "{t.title}" }
                                }
                            }
                            for c in cards {
                                div { key: "{c.card.id}", class: "flex items-center gap-1.5 px-1 text-xs text-muted-foreground",
                                    span { class: "size-2.5 shrink-0 rounded-[2px] border border-muted-foreground" }
                                    span { class: "truncate", "{c.card.title}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
