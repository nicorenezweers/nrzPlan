use dioxus::prelude::*;
use icons::{ArrowRight, Pencil, Plus, Trash2, X, Zap};

use crate::api::get_activity;
use crate::components::ui::skeleton::Skeleton;
use crate::util::fmt_ago;

#[component]
fn KindIcon(kind: String) -> Element {
    let c = "size-3.5";
    match kind.rsplit('.').next().unwrap_or_default() {
        "created" => rsx! { Plus { class: c } },
        "moved" => rsx! { ArrowRight { class: c } },
        "deleted" => rsx! { Trash2 { class: c } },
        _ => rsx! { Pencil { class: c } },
    }
}

/// Seitenleiste mit dem Aktivitätslog eines Boards (UI- und API-Änderungen).
#[component]
pub fn ActivityPanel(board_id: String, version: u64, onclose: EventHandler<()>) -> Element {
    let list = use_resource(use_reactive!(|(board_id, version)| async move {
        let _ = version;
        get_activity(Some(board_id), 100).await
    }));

    rsx! {
        div { class: "fixed inset-0 z-[150] bg-black/30 sm:bg-transparent", onclick: move |_| onclose.call(()) }
        aside {
            class: "fixed inset-y-0 right-0 z-[160] flex w-full max-w-sm flex-col border-l bg-popover shadow-2xl",
            div { class: "flex items-center justify-between border-b px-5 py-3",
                h2 { class: "text-sm font-semibold", "Aktivität" }
                button { class: "rounded-md p-1 text-muted-foreground hover:bg-accent hover:text-foreground", onclick: move |_| onclose.call(()),
                    X { class: "size-4" }
                }
            }
            div { class: "flex-1 overflow-y-auto px-3 py-3",
                match &*list.read() {
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        p { class: "px-2 py-6 text-center text-sm text-muted-foreground", "Noch keine Aktivität." }
                    },
                    Some(Ok(items)) => rsx! {
                        ol { class: "relative flex flex-col gap-0.5",
                            for a in items.iter() {
                                li { key: "{a.id}", class: "flex gap-3 rounded-lg px-2 py-2 hover:bg-accent/30",
                                    div { class: "mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground",
                                        KindIcon { kind: a.kind.clone() }
                                    }
                                    div { class: "min-w-0 flex-1",
                                        p { class: "text-sm leading-snug", "{a.message}" }
                                        p { class: "mt-0.5 flex items-center gap-1.5 text-[11px] text-muted-foreground",
                                            if a.actor.starts_with("API") {
                                                span { class: "inline-flex items-center gap-1 rounded bg-muted px-1 py-px",
                                                    Zap { class: "size-3" }
                                                    "{a.actor}"
                                                }
                                            }
                                            span { "{fmt_ago(a.created_at)}" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { p { class: "text-sm text-destructive", "Fehler: {e}" } },
                    None => rsx! {
                        div { class: "flex flex-col gap-2",
                            Skeleton { class: "h-10" }
                            Skeleton { class: "h-10" }
                        }
                    },
                }
            }
        }
    }
}
