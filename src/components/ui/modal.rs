use dioxus::prelude::*;
use icons::X;
use tw_merge::tw_merge;

/// Einfacher Dialog (Overlay). Schliesst per Klick auf den Hintergrund, X oder Escape.
#[component]
pub fn Modal(
    #[props(into, optional)] title: Option<String>,
    #[props(into, optional)] class: Option<String>,
    onclose: EventHandler<()>,
    children: Element,
) -> Element {
    let panel = tw_merge!(
        "relative flex max-h-[90dvh] w-full max-w-lg flex-col overflow-hidden rounded-xl border bg-popover text-popover-foreground shadow-2xl",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div {
            class: "fixed inset-0 z-[200] flex items-end justify-center bg-black/60 p-0 backdrop-blur-[2px] sm:items-center sm:p-6",
            tabindex: "-1",
            onmounted: move |e| async move { let _ = e.set_focus(true).await; },
            onkeydown: move |e| if e.key() == Key::Escape { onclose.call(()) },
            onclick: move |_| onclose.call(()),
            div { class: "{panel}", onclick: move |e| e.stop_propagation(),
                if let Some(t) = &title {
                    div { class: "flex items-center justify-between border-b px-5 py-3",
                        h2 { class: "text-sm font-semibold", "{t}" }
                        button {
                            class: "rounded-md p-1 text-muted-foreground hover:bg-accent hover:text-foreground",
                            onclick: move |_| onclose.call(()),
                            X { class: "size-4" }
                        }
                    }
                }
                div { class: "overflow-y-auto", {children} }
            }
        }
    }
}
