use dioxus::prelude::*;

/// Gemeinsame Klassen für native Formularelemente (im nrz-Stil).
pub const INPUT: &str = "flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring [color-scheme:dark]";
pub const TEXTAREA: &str = "flex min-h-24 w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring";
pub const SELECT: &str = "flex h-9 w-full rounded-md border border-input bg-background px-2 py-1 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring";
/// Unsichtbares Input (z.B. Titel inline bearbeiten).
pub const GHOST_INPUT: &str = "w-full bg-transparent outline-none placeholder:text-muted-foreground";

#[component]
pub fn Field(#[props(into)] label: String, children: Element) -> Element {
    rsx! {
        label { class: "flex flex-col gap-1.5",
            span { class: "text-xs font-medium text-muted-foreground", "{label}" }
            {children}
        }
    }
}
