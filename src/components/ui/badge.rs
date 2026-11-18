use dioxus::prelude::*;
use tw_merge::tw_merge;

use crate::util::dot_class;

/// Label-Pille mit farbigem Punkt (wie in Kanban-Boards üblich).
#[component]
pub fn LabelBadge(
    #[props(into)] name: String,
    #[props(into)] color: String,
    #[props(into, optional)] class: Option<String>,
) -> Element {
    let merged = tw_merge!(
        "inline-flex items-center gap-1.5 rounded-full border border-border px-2 py-0.5 text-[11px] font-medium text-foreground/90 whitespace-nowrap",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        span { class: "{merged}",
            span { class: "size-1.5 rounded-full {dot_class(&color)}" }
            "{name}"
        }
    }
}

/// Kleine neutrale Info-Pille.
#[component]
pub fn Pill(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "inline-flex items-center gap-1 rounded-md bg-muted px-1.5 py-0.5 text-[11px] text-muted-foreground whitespace-nowrap",
        class.as_deref().unwrap_or("")
    );
    rsx! { span { class: "{merged}", {children} } }
}
