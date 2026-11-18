use dioxus::prelude::*;

use crate::components::hooks::use_theme_mode::use_theme_provider;
use crate::components::layout::app_bottom_nav::AppBottomNav;
use crate::components::layout::app_wrapper::AppWrapper;
use crate::components::layout::command_palette::{CommandPalette, Palette, Refresh};
use crate::components::layout::sidebar::Sidebar;
use crate::domain::boards::page_board::BoardPage;
use crate::domain::boards::page_boards::BoardsPage;
use crate::domain::calendar::page_calendar::CalendarPage;
use crate::domain::settings::page_settings::SettingsPage;
use crate::domain::todos::page_todos::TodosPage;
use crate::domain::today::page_today::TodayPage;

const TAILWIND_CSS: Asset = asset!("/public/tailwind.css");
const FAVICON: Asset = asset!("/public/icons/favicon-32x32.png");

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
        #[route("/")]
        Today {},
        #[route("/todos")]
        Todos {},
        #[route("/kalender")]
        Calendar {},
        #[route("/boards")]
        Boards {},
        #[route("/boards/:id")]
        Board { id: String },
        #[route("/einstellungen")]
        Settings {},
    #[end_layout]
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

#[component]
pub fn App() -> Element {
    use_theme_provider();

    rsx! {
        document::Title { "nrzPlan" }
        document::Link { rel: "icon", href: FAVICON }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1, viewport-fit=cover" }
        document::Meta { name: "theme-color", content: "#1c1c1c" }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

#[component]
fn AppLayout() -> Element {
    use_context_provider(|| Palette(Signal::new(false)));
    use_context_provider(|| Refresh(Signal::new(0)));
    rsx! {
        CommandPalette {}
        AppWrapper {
            div { class: "flex h-full min-h-0 flex-1",
                Sidebar {}
                main { class: "flex min-w-0 flex-1 flex-col overflow-y-auto overflow-x-clip pb-20 sm:pb-0",
                    Outlet::<Route> {}
                }
            }
            AppBottomNav {}
        }
    }
}

#[component]
fn Today() -> Element {
    rsx! { TodayPage {} }
}
#[component]
fn Todos() -> Element {
    rsx! { TodosPage {} }
}
#[component]
fn Calendar() -> Element {
    rsx! { CalendarPage {} }
}
#[component]
fn Boards() -> Element {
    rsx! { BoardsPage {} }
}
#[component]
fn Board(id: String) -> Element {
    rsx! { BoardPage { key: "{id}", id } }
}
#[component]
fn Settings() -> Element {
    rsx! { SettingsPage {} }
}

#[component]
fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "flex h-full flex-col items-center justify-center gap-4 p-8",
            p { class: "text-2xl font-semibold", "404" }
            p { class: "text-muted-foreground", "Seite nicht gefunden: /{route.join(\"/\")}" }
            Link { to: Route::Today {}, class: "text-sm underline", "Zur Übersicht" }
        }
    }
}
