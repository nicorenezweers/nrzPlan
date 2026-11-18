use dioxus::prelude::*;
use icons::{CalendarDays, ListTodo, Settings, SquareKanban, Sun};

use crate::app::Route;
use crate::components::hooks::use_is_current_path::use_is_current_path;
use crate::components::ui::bottom_nav::{BottomNav, BottomNavButton, BottomNavGrid, BottomNavLabel};

#[component]
pub fn AppBottomNav() -> Element {
    let is_current = use_is_current_path();
    let navigator = use_navigator();

    rsx! {
        BottomNav {
            BottomNavGrid { class: "grid-cols-5",
                BottomNavButton { active: is_current("/"), onclick: move |_| { navigator.push(Route::Today {}); },
                    Sun { class: "size-5" }
                    BottomNavLabel { "Heute" }
                }
                BottomNavButton { active: is_current("/todos"), onclick: move |_| { navigator.push(Route::Todos {}); },
                    ListTodo { class: "size-5" }
                    BottomNavLabel { "Todos" }
                }
                BottomNavButton { active: is_current("/kalender"), onclick: move |_| { navigator.push(Route::Calendar {}); },
                    CalendarDays { class: "size-5" }
                    BottomNavLabel { "Kalender" }
                }
                BottomNavButton { active: is_current("/boards"), onclick: move |_| { navigator.push(Route::Boards {}); },
                    SquareKanban { class: "size-5" }
                    BottomNavLabel { "Boards" }
                }
                BottomNavButton { active: is_current("/einstellungen"), onclick: move |_| { navigator.push(Route::Settings {}); },
                    Settings { class: "size-5" }
                    BottomNavLabel { "Mehr" }
                }
            }
        }
    }
}
