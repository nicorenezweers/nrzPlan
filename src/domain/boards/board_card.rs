use dioxus::prelude::*;
use icons::{AlignLeft, Calendar, Flag, Link, Zap};

use crate::components::ui::badge::{LabelBadge, Pill};
use crate::model::{Card, Label};
use crate::util::{fmt_relative, priority_class, today};

pub fn label_color(labels: &[Label], name: &str) -> String {
    labels.iter().find(|l| l.name.eq_ignore_ascii_case(name)).map(|l| l.color.clone()).unwrap_or_else(|| "gray".into())
}

/// Kreis-Fortschritt wie "0/4" im Screenshot.
#[component]
pub fn ChecklistProgress(done: i64, total: i64) -> Element {
    let pct = if total > 0 { done as f64 / total as f64 } else { 0.0 };
    let circ = 2.0 * std::f64::consts::PI * 5.0;
    let offset = circ * (1.0 - pct);
    let complete = total > 0 && done == total;
    rsx! {
        span { class: if complete { "inline-flex items-center gap-1.5 rounded-full border border-emerald-500/40 px-2 py-0.5 text-[11px] text-emerald-400" } else { "inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[11px] text-muted-foreground" },
            svg { class: "size-3 -rotate-90", view_box: "0 0 12 12",
                circle { cx: "6", cy: "6", r: "5", fill: "none", stroke: "currentColor", stroke_opacity: "0.25", stroke_width: "1.5" }
                circle {
                    cx: "6",
                    cy: "6",
                    r: "5",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    stroke_dasharray: "{circ}",
                    stroke_dashoffset: "{offset}",
                    stroke_linecap: "round",
                }
            }
            "{done}/{total}"
        }
    }
}

#[component]
pub fn BoardCard(
    card: Card,
    labels: Vec<Label>,
    in_done_column: bool,
    dragging: bool,
    drop_before: bool,
    onopen: EventHandler<()>,
    ondragstart: EventHandler<DragEvent>,
    ondragend: EventHandler<()>,
    ondragover: EventHandler<DragEvent>,
    ondrop: EventHandler<DragEvent>,
) -> Element {
    let overdue = !in_done_column && card.due_date.is_some_and(|d| d < today());
    let has_footer = card.description.is_some()
        || card.checklist_total > 0
        || card.due_date.is_some()
        || card.priority > 0
        || card.source.is_some()
        || card.url.is_some();

    rsx! {
        div {
            class: "relative",
            ondragover: move |e| {
                e.prevent_default();
                ondragover.call(e);
            },
            ondrop: move |e| {
                e.prevent_default();
                e.stop_propagation();
                ondrop.call(e);
            },
            if drop_before {
                div { class: "pointer-events-none absolute -top-[5px] inset-x-1 h-0.5 rounded-full bg-foreground/60" }
            }
            div {
                draggable: "true",
                class: if dragging { "cursor-grab rounded-lg border border-dashed border-border bg-card/40 p-3 opacity-40" } else { "group cursor-pointer rounded-lg border border-transparent bg-card p-3 shadow-sm transition-colors hover:border-border hover:bg-accent/40" },
                onclick: move |_| onopen.call(()),
                ondragstart: move |e| ondragstart.call(e),
                ondragend: move |_| ondragend.call(()),
                p { class: if in_done_column { "text-sm leading-snug text-foreground/80" } else { "text-sm leading-snug" }, "{card.title}" }
                if !card.labels.is_empty() {
                    div { class: "mt-2 flex flex-wrap gap-1.5",
                        for l in card.labels.iter() {
                            LabelBadge { key: "{l}", name: l.clone(), color: label_color(&labels, l) }
                        }
                    }
                }
                if has_footer {
                    div { class: "mt-2.5 flex items-center gap-2 text-muted-foreground",
                        if card.description.is_some() { AlignLeft { class: "size-3.5" } }
                        if card.url.is_some() { Link { class: "size-3.5" } }
                        if card.priority > 0 { Flag { class: "size-3.5 {priority_class(card.priority)}" } }
                        if let Some(s) = &card.source {
                            Pill { Zap { class: "size-3" } "{s}" }
                        }
                        div { class: "flex-1" }
                        if let Some(d) = card.due_date {
                            Pill { class: if overdue { "bg-red-500/10 text-red-400" } else { "" },
                                Calendar { class: "size-3" }
                                "{fmt_relative(d)}"
                            }
                        }
                        if card.checklist_total > 0 {
                            ChecklistProgress { done: card.checklist_done, total: card.checklist_total }
                        }
                    }
                }
            }
        }
    }
}
