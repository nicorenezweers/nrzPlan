use dioxus::prelude::*;
use icons::{Plus, SquareKanban};

use crate::api::{create_board, get_boards};
use crate::app::Route;
use crate::components::layout::page::PageHeader;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::field::INPUT;
use crate::components::ui::skeleton::Skeleton;

#[component]
pub fn BoardsPage() -> Element {
    let boards = use_server_future(get_boards)?;
    let mut creating = use_signal(|| false);
    let mut name = use_signal(String::new);
    let navigator = use_navigator();

    let submit = move |e: FormEvent| {
        e.prevent_default();
        let n = name().trim().to_string();
        if n.is_empty() {
            return;
        }
        spawn(async move {
            if let Ok(b) = create_board(n).await {
                navigator.push(Route::Board { id: b.id });
            }
        });
    };

    rsx! {
        div { class: "mx-auto flex w-full max-w-5xl flex-col",
            PageHeader {
                title: "Boards",
                subtitle: "Projekte, Roadmaps und Abläufe als Kanban",
                actions: rsx! {
                    Button { onclick: move |_| creating.set(true),
                        Plus { class: "size-4" }
                        "Neues Board"
                    }
                },
            }
            div { class: "flex flex-col gap-4 px-4 pb-10 sm:px-8",
                if creating() {
                    form { class: "flex max-w-md gap-2", onsubmit: submit,
                        input {
                            class: INPUT,
                            placeholder: "Name, z.B. Roadmap",
                            value: name(),
                            onmounted: move |e| async move { let _ = e.set_focus(true).await; },
                            oninput: move |e| name.set(e.value()),
                        }
                        Button { submit: true, "Anlegen" }
                        Button { variant: ButtonVariant::Ghost, onclick: move |_| creating.set(false), "Abbrechen" }
                    }
                    p { class: "text-xs text-muted-foreground", "Startet mit den Listen Backlog, Geplant, In Arbeit und Erledigt." }
                }
                match &*boards.read() {
                    Some(Ok(list)) if list.is_empty() => rsx! {
                        Empty {
                            EmptyTitle { "Noch keine Boards" }
                            EmptyDescription { "Lege dein erstes Board an – z.B. für ein Projekt oder deine Roadmap." }
                            Button { size: ButtonSize::Sm, class: "mt-3", onclick: move |_| creating.set(true), "Board anlegen" }
                        }
                    },
                    Some(Ok(list)) => rsx! {
                        div { class: "grid gap-3 sm:grid-cols-2 lg:grid-cols-3",
                            for b in list.iter() {
                                Link {
                                    key: "{b.id}",
                                    to: Route::Board { id: b.id.clone() },
                                    class: "group flex flex-col gap-3 rounded-xl border bg-card/60 p-4 transition-colors hover:bg-accent/40",
                                    div { class: "flex items-center gap-3",
                                        div { class: "flex size-9 items-center justify-center rounded-lg bg-muted text-muted-foreground group-hover:text-foreground",
                                            SquareKanban { class: "size-4" }
                                        }
                                        div { class: "min-w-0",
                                            p { class: "truncate font-medium", "{b.name}" }
                                            p { class: "text-xs text-muted-foreground", "{b.open_count} offen · {b.card_count} Karten" }
                                        }
                                    }
                                    if b.card_count > 0 {
                                        div { class: "h-1 overflow-hidden rounded-full bg-muted",
                                            div {
                                                class: "h-full rounded-full bg-foreground/70",
                                                style: "width: {(b.card_count - b.open_count) * 100 / b.card_count}%",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { p { class: "text-sm text-destructive", "Fehler: {e}" } },
                    None => rsx! {
                        div { class: "grid gap-3 sm:grid-cols-3",
                            Skeleton { class: "h-24" }
                            Skeleton { class: "h-24" }
                            Skeleton { class: "h-24" }
                        }
                    },
                }
            }
        }
    }
}
