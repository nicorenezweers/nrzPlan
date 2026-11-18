use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dioxus::prelude::*;
use icons::Trash2;

use crate::api::{create_event, delete_event, update_event};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::field::{Field, INPUT, TEXTAREA};
use crate::components::ui::modal::Modal;
use crate::model::{EVENT_COLORS, Event, EventPatch, NewEvent};
use crate::util::{date_input, dot_class, parse_date};

fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M").ok()
}

fn combine(d: &str, t: &str, fallback: NaiveTime) -> Option<NaiveDateTime> {
    Some(parse_date(d)?.and_time(parse_time(t).unwrap_or(fallback)))
}

/// Termin anlegen (`event = None`) oder bearbeiten. `onclose(true)` = neu laden.
#[component]
pub fn EventDialog(event: Option<Event>, date: NaiveDate, onclose: EventHandler<bool>) -> Element {
    let ev = event.clone();
    let start0 = ev.as_ref().map(|e| e.start_at).unwrap_or_else(|| {
        let hour = chrono::Timelike::hour(&chrono::Local::now()).clamp(7, 21) + 1;
        date.and_hms_opt(hour.min(23), 0, 0).unwrap_or_default()
    });
    let end0 = ev.as_ref().map(|e| e.end_at).unwrap_or(start0 + chrono::Duration::hours(1));

    let mut title = use_signal(|| ev.as_ref().map(|e| e.title.clone()).unwrap_or_default());
    let mut all_day = use_signal(|| ev.as_ref().is_some_and(|e| e.all_day));
    let mut start_date = use_signal(|| date_input(Some(start0.date())));
    let mut start_time = use_signal(|| start0.format("%H:%M").to_string());
    let mut end_date = use_signal(|| date_input(Some(end0.date())));
    let mut end_time = use_signal(|| end0.format("%H:%M").to_string());
    let mut location = use_signal(|| ev.as_ref().and_then(|e| e.location.clone()).unwrap_or_default());
    let mut description = use_signal(|| ev.as_ref().and_then(|e| e.description.clone()).unwrap_or_default());
    let mut color = use_signal(|| ev.as_ref().map(|e| e.color.clone()).unwrap_or_else(|| "blue".into()));
    let mut error = use_signal(|| None::<String>);

    let existing_id = ev.as_ref().map(|e| e.id.clone());
    let id_del = existing_id.clone();

    let save = move |e: FormEvent| {
        e.prevent_default();
        let Some(start) = combine(&start_date(), &start_time(), NaiveTime::MIN) else {
            error.set(Some("Startdatum fehlt".into()));
            return;
        };
        let end = combine(&end_date(), &end_time(), NaiveTime::MIN).filter(|e| *e > start || all_day());
        if title().trim().is_empty() {
            error.set(Some("Titel fehlt".into()));
            return;
        }
        let id = existing_id.clone();
        spawn(async move {
            let res = match id {
                Some(id) => update_event(
                    id,
                    EventPatch {
                        title: Some(title()),
                        description: Some(Some(description())),
                        location: Some(Some(location())),
                        start_at: Some(start),
                        end_at: Some(end.unwrap_or(start + chrono::Duration::hours(1))),
                        all_day: Some(all_day()),
                        color: Some(color()),
                    },
                )
                .await
                .map(|_| ()),
                None => create_event(NewEvent {
                    title: title(),
                    description: Some(description()),
                    location: Some(location()),
                    start_at: start,
                    end_at: end,
                    all_day: all_day(),
                    color: Some(color()),
                    ..Default::default()
                })
                .await
                .map(|_| ()),
            };
            match res {
                Ok(()) => onclose.call(true),
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    rsx! {
        Modal { title: if event.is_some() { "Termin bearbeiten" } else { "Neuer Termin" }, onclose: move |_| onclose.call(false),
            form { onsubmit: save,
                div { class: "flex flex-col gap-4 p-5",
                    input {
                        class: "w-full bg-transparent text-lg font-semibold outline-none placeholder:text-muted-foreground",
                        placeholder: "Titel",
                        value: title(),
                        onmounted: move |e| async move { let _ = e.set_focus(true).await; },
                        oninput: move |e| title.set(e.value()),
                    }
                    label { class: "flex items-center gap-2 text-sm",
                        input { r#type: "checkbox", class: "size-4 accent-neutral-300", checked: all_day(), onchange: move |e| all_day.set(e.checked()) }
                        "Ganztägig"
                    }
                    div { class: "grid grid-cols-2 gap-3",
                        Field { label: "Beginn",
                            div { class: "flex gap-2",
                                input { r#type: "date", class: INPUT, value: start_date(), oninput: move |e| {
                                    // Ende mitziehen, wenn es vor dem Start liegt
                                    if end_date() < e.value() { end_date.set(e.value()); }
                                    start_date.set(e.value());
                                } }
                                if !all_day() {
                                    input { r#type: "time", class: "{INPUT} w-28", value: start_time(), oninput: move |e| start_time.set(e.value()) }
                                }
                            }
                        }
                        Field { label: "Ende",
                            div { class: "flex gap-2",
                                input { r#type: "date", class: INPUT, value: end_date(), oninput: move |e| end_date.set(e.value()) }
                                if !all_day() {
                                    input { r#type: "time", class: "{INPUT} w-28", value: end_time(), oninput: move |e| end_time.set(e.value()) }
                                }
                            }
                        }
                    }
                    Field { label: "Ort",
                        input { class: INPUT, placeholder: "Optional", value: location(), oninput: move |e| location.set(e.value()) }
                    }
                    Field { label: "Notizen",
                        textarea { class: TEXTAREA, value: description(), oninput: move |e| description.set(e.value()) }
                    }
                    div { class: "flex items-center gap-2",
                        span { class: "mr-1 text-xs font-medium text-muted-foreground", "Farbe" }
                        for c in EVENT_COLORS {
                            button {
                                key: "{c}",
                                r#type: "button",
                                class: if color() == c { "size-5 rounded-full ring-2 ring-foreground ring-offset-2 ring-offset-popover {dot_class(c)}" } else { "size-5 rounded-full opacity-70 hover:opacity-100 {dot_class(c)}" },
                                onclick: move |_| color.set(c.to_string()),
                            }
                        }
                    }
                    if let Some(s) = event.as_ref().and_then(|e| e.source.clone()) {
                        p { class: "text-xs text-muted-foreground", "Extern angelegt über {s}" }
                    }
                    if let Some(err) = error() {
                        p { class: "text-sm text-destructive", "{err}" }
                    }
                }
                div { class: "sticky bottom-0 flex items-center gap-2 border-t bg-popover px-5 py-3",
                    if let Some(id) = id_del.clone() {
                        Button {
                            variant: ButtonVariant::Ghost,
                            class: "text-muted-foreground hover:text-destructive",
                            onclick: move |e: MouseEvent| {
                                e.prevent_default();
                                let id = id.clone();
                                spawn(async move {
                                    if delete_event(id).await.is_ok() {
                                        onclose.call(true);
                                    }
                                });
                            },
                            Trash2 { class: "size-4" }
                            "Löschen"
                        }
                    }
                    div { class: "flex-1" }
                    Button { variant: ButtonVariant::Ghost, onclick: move |e: MouseEvent| { e.prevent_default(); onclose.call(false); }, "Abbrechen" }
                    Button { submit: true, "Speichern" }
                }
            }
        }
    }
}
