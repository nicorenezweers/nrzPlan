use dioxus::prelude::*;
use icons::{CalendarDays, ListTodo, Search, Settings, SquareKanban, Sun};

use crate::app::Route;
use crate::components::hooks::use_is_current_path::use_is_current_path;
use crate::components::layout::command_palette::Palette;
use crate::components::layout::theme_toggle::ThemeToggle;

/// Schmale Icon-Leiste links (Desktop).
#[component]
pub fn Sidebar() -> Element {
    let is_current = use_is_current_path();
    let mut palette = use_context::<Palette>().0;
    let item = |active: bool| {
        if active {
            "bg-accent text-foreground"
        } else {
            "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
        }
    };
    rsx! {
        aside { class: "hidden sm:flex w-14 shrink-0 flex-col items-center gap-1 border-r bg-sidebar/40 py-3",
            Link {
                to: Route::Today {},
                class: "mb-3 flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground text-sm font-bold",
                title: "nrzPlan",
                "P"
            }
            button {
                title: "Suchen (⌘K)",
                class: "mb-2 rounded-lg p-2 text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground",
                onclick: move |_| palette.set(true),
                Search { class: "size-[18px]" }
            }
            Link { to: Route::Today {}, title: "Heute", class: "rounded-lg p-2 transition-colors {item(is_current(\"/\"))}",
                Sun { class: "size-[18px]" }
            }
            Link { to: Route::Todos {}, title: "Todos", class: "rounded-lg p-2 transition-colors {item(is_current(\"/todos\"))}",
                ListTodo { class: "size-[18px]" }
            }
            Link { to: Route::Calendar {}, title: "Kalender", class: "rounded-lg p-2 transition-colors {item(is_current(\"/kalender\"))}",
                CalendarDays { class: "size-[18px]" }
            }
            Link { to: Route::Boards {}, title: "Boards", class: "rounded-lg p-2 transition-colors {item(is_current(\"/boards\"))}",
                SquareKanban { class: "size-[18px]" }
            }
            div { class: "flex-1" }
            ThemeToggle {}
            Link { to: Route::Settings {}, title: "Einstellungen", class: "rounded-lg p-2 transition-colors {item(is_current(\"/einstellungen\"))}",
                Settings { class: "size-[18px]" }
            }
        }
    }
}
