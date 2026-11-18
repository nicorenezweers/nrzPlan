use dioxus::prelude::*;
use icons::Check;

/// Runde Abhak-Box für Todos; Randfarbe zeigt die Priorität.
#[component]
pub fn TodoCheck(checked: bool, #[props(default)] priority: i64, onclick: EventHandler<MouseEvent>) -> Element {
    let ring = match priority {
        3 => "border-red-400",
        2 => "border-orange-400",
        1 => "border-sky-400",
        _ => "border-muted-foreground/60",
    };
    let state = if checked { "bg-muted-foreground border-muted-foreground text-background" } else { "hover:bg-accent" };
    rsx! {
        button {
            r#type: "button",
            class: "flex size-[18px] shrink-0 items-center justify-center rounded-full border-[1.5px] transition-colors {ring} {state}",
            onclick: move |e| {
                e.stop_propagation();
                onclick.call(e);
            },
            if checked { Check { class: "size-3" } }
        }
    }
}
