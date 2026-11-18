use dioxus::prelude::*;
use tw_merge::tw_merge;

/// Seitenkopf: Titel links, Aktionen rechts (wie im Board-Screenshot).
#[component]
pub fn PageHeader(
    #[props(into)] title: String,
    #[props(into, optional)] subtitle: Option<String>,
    #[props(optional)] actions: Option<Element>,
) -> Element {
    rsx! {
        header { class: "flex flex-wrap items-center gap-3 px-4 pb-4 pt-[calc(env(safe-area-inset-top)+1.25rem)] sm:px-8 sm:pt-7",
            div { class: "min-w-0 flex-1",
                h1 { class: "truncate text-xl font-semibold tracking-tight sm:text-2xl", "{title}" }
                if let Some(s) = &subtitle {
                    p { class: "mt-0.5 text-sm text-muted-foreground", "{s}" }
                }
            }
            if let Some(a) = actions {
                div { class: "flex flex-wrap items-center gap-2", {a} }
            }
        }
    }
}

/// Sektion mit Überschrift und optionalem Zähler.
#[component]
pub fn Section(
    #[props(into)] title: String,
    #[props(optional)] count: Option<usize>,
    #[props(into, optional)] class: Option<String>,
    #[props(into, optional)] title_class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!("flex flex-col gap-1", class.as_deref().unwrap_or(""));
    let t = tw_merge!(
        "flex items-center gap-2 px-1 pb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground",
        title_class.as_deref().unwrap_or("")
    );
    rsx! {
        section { class: "{merged}",
            h2 { class: "{t}",
                "{title}"
                if let Some(c) = count {
                    span { class: "rounded bg-muted px-1.5 py-px text-[10px] font-medium normal-case", "{c}" }
                }
            }
            {children}
        }
    }
}
