use dioxus::prelude::*;
use icons::{ChevronLeft, Ellipsis, Funnel, History, Link as LinkIcon, Plus, X};

use crate::api::{
    create_card, create_column, delete_board, delete_column, get_board, move_card, move_column, update_board,
    update_column,
};
use crate::app::Route;
use crate::components::ui::badge::LabelBadge;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{GHOST_INPUT, INPUT};
use crate::components::ui::skeleton::Skeleton;
use crate::domain::boards::activity_panel::ActivityPanel;
use crate::domain::boards::board_card::BoardCard;
use crate::domain::boards::card_dialog::{CardDialog, copy_to_clipboard};
use crate::model::{BoardFull, Card, Column, NewCard};
use crate::util::{parse_quick_add, today};

#[derive(Clone, PartialEq)]
struct DropTarget {
    column_id: String,
    index: usize,
}

#[component]
pub fn BoardPage(id: String) -> Element {
    let board_id = id.clone();
    let mut res = use_server_future(move || get_board(board_id.clone()))?;
    // Lokale Kopie für sofortiges Feedback beim Verschieben.
    let mut local = use_signal(|| None::<BoardFull>);
    use_effect(move || {
        if let Some(Ok(b)) = &*res.read() {
            local.set(Some(b.clone()));
        }
    });

    let mut dragging = use_signal(|| None::<String>);
    let mut drop_target = use_signal(|| None::<DropTarget>);
    let mut open_card = use_signal(|| None::<String>);
    let mut show_filter = use_signal(|| false);
    let mut search = use_signal(String::new);
    let mut label_filter = use_signal(|| None::<String>);
    let mut adding_column = use_signal(|| false);
    let mut new_column = use_signal(String::new);
    let mut menu_open = use_signal(|| false);
    let mut show_activity = use_signal(|| false);
    // Ändert sich bei jedem Neuladen → Aktivitäts-Panel lädt mit.
    let mut version = use_signal(|| 0u64);
    use_effect(move || {
        if res.read().is_some() {
            version += 1;
        }
    });
    let mut renaming = use_signal(|| false);
    let mut rename_value = use_signal(String::new);
    let navigator = use_navigator();

    let full = local().or_else(|| match &*res.read() {
        Some(Ok(b)) => Some(b.clone()),
        _ => None,
    });
    let error = match &*res.read() {
        Some(Err(e)) => Some(e.to_string()),
        _ => None,
    };

    let Some(full) = full else {
        return rsx! {
            div { class: "flex flex-col gap-4 p-8",
                if let Some(e) = error {
                    p { class: "text-sm text-destructive", "Board konnte nicht geladen werden: {e}" }
                    Link { to: Route::Boards {}, class: "text-sm underline", "Zurück zu den Boards" }
                } else {
                    Skeleton { class: "h-8 w-48" }
                    div { class: "flex gap-4",
                        Skeleton { class: "h-96 w-72" }
                        Skeleton { class: "h-96 w-72" }
                        Skeleton { class: "h-96 w-72" }
                    }
                }
            }
        };
    };

    let board = full.board.clone();
    let columns = full.columns.clone();
    let labels = full.labels.clone();
    let q = search().to_lowercase();
    let matches = move |c: &Card| -> bool {
        (q.is_empty()
            || c.title.to_lowercase().contains(&q)
            || c.description.as_deref().is_some_and(|d| d.to_lowercase().contains(&q)))
            && label_filter().is_none_or(|l| c.labels.iter().any(|x| x.eq_ignore_ascii_case(&l)))
    };

    // Karte lokal verschieben + Server aktualisieren.
    let mut do_move = move |card_id: String, column_id: String, index: usize| {
        let mut moved_index = index;
        local.with_mut(|b| {
            let Some(b) = b else { return };
            let Some(pos) = b.cards.iter().position(|c| c.id == card_id) else { return };
            let mut card = b.cards.remove(pos);
            card.column_id = column_id.clone();
            let mut col: Vec<Card> = b.cards.iter().filter(|c| c.column_id == column_id).cloned().collect();
            b.cards.retain(|c| c.column_id != column_id);
            moved_index = index.min(col.len());
            col.insert(moved_index, card);
            for (i, c) in col.iter_mut().enumerate() {
                c.position = i as i64;
            }
            b.cards.extend(col);
        });
        spawn(async move {
            let _ = move_card(card_id, column_id, moved_index as i64).await;
            res.restart();
        });
    };

    let reload = move |_| res.restart();
    let api_path = format!("/api/v1/boards/{}", board.id);
    let bid_rename = board.id.clone();
    let bid_delete = board.id.clone();
    let bid_col = board.id.clone();

    rsx! {
        div { class: "flex h-full min-h-0 flex-col",
            // Kopfzeile
            header { class: "flex flex-wrap items-center gap-3 px-4 pb-4 pt-[calc(env(safe-area-inset-top)+1.25rem)] sm:px-8 sm:pt-7",
                Link { to: Route::Boards {}, class: "rounded-md p-1 text-muted-foreground hover:bg-accent hover:text-foreground sm:hidden",
                    ChevronLeft { class: "size-5" }
                }
                if renaming() {
                    form {
                        class: "min-w-0 flex-1",
                        onsubmit: move |e| {
                            e.prevent_default();
                            let id = bid_rename.clone();
                            let name = rename_value();
                            renaming.set(false);
                            spawn(async move {
                                let _ = update_board(id, name, None).await;
                                res.restart();
                            });
                        },
                        input {
                            class: "{GHOST_INPUT} text-xl font-semibold sm:text-2xl",
                            value: rename_value(),
                            onmounted: move |e| async move { let _ = e.set_focus(true).await; },
                            oninput: move |e| rename_value.set(e.value()),
                            onblur: move |_| renaming.set(false),
                        }
                    }
                } else {
                    h1 {
                        class: "min-w-0 flex-1 cursor-text truncate text-xl font-semibold tracking-tight sm:text-2xl",
                        ondoubleclick: {
                            let n = board.name.clone();
                            move |_| {
                                rename_value.set(n.clone());
                                renaming.set(true);
                            }
                        },
                        "{board.name}"
                    }
                }
                div { class: "flex flex-wrap items-center gap-2",
                    button {
                        class: "hidden items-center gap-2 rounded-full border px-4 py-1.5 font-mono text-xs text-muted-foreground hover:text-foreground md:flex",
                        title: "API-Pfad kopieren",
                        onclick: {
                            let p = api_path.clone();
                            move |_| copy_to_clipboard(p.clone())
                        },
                        span { "nrzplan" }
                        span { class: "opacity-40", "/" }
                        span { "boards" }
                        span { class: "opacity-40", "/" }
                        span { class: "max-w-40 truncate", "{board.name.to_lowercase()}" }
                        LinkIcon { class: "size-3.5" }
                    }
                    Button {
                        variant: if show_filter() || label_filter().is_some() || !search().is_empty() { ButtonVariant::Secondary } else { ButtonVariant::Outline },
                        onclick: move |_| show_filter.toggle(),
                        Funnel { class: "size-4" }
                        span { class: "hidden sm:inline", "Filter" }
                    }
                    Button {
                        variant: if show_activity() { ButtonVariant::Secondary } else { ButtonVariant::Outline },
                        onclick: move |_| show_activity.toggle(),
                        History { class: "size-4" }
                        span { class: "hidden lg:inline", "Aktivität" }
                    }
                    Button { variant: ButtonVariant::Default, onclick: move |_| adding_column.set(true),
                        Plus { class: "size-4" }
                        span { class: "hidden sm:inline", "Neue Liste" }
                    }
                    div { class: "relative",
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, onclick: move |_| menu_open.toggle(),
                            Ellipsis { class: "size-4" }
                        }
                        if menu_open() {
                            div { class: "fixed inset-0 z-40", onclick: move |_| menu_open.set(false) }
                            div { class: "absolute right-0 top-10 z-50 w-48 rounded-lg border bg-popover p-1 text-sm shadow-xl",
                                button {
                                    class: "w-full rounded-md px-3 py-1.5 text-left hover:bg-accent",
                                    onclick: {
                                        let n = board.name.clone();
                                        move |_| {
                                            menu_open.set(false);
                                            rename_value.set(n.clone());
                                            renaming.set(true);
                                        }
                                    },
                                    "Umbenennen"
                                }
                                button {
                                    class: "w-full rounded-md px-3 py-1.5 text-left hover:bg-accent",
                                    onclick: {
                                        let p = api_path.clone();
                                        move |_| {
                                            menu_open.set(false);
                                            copy_to_clipboard(p.clone());
                                        }
                                    },
                                    "API-Pfad kopieren"
                                }
                                button {
                                    class: "w-full rounded-md px-3 py-1.5 text-left text-destructive hover:bg-destructive/10",
                                    onclick: move |_| {
                                        let id = bid_delete.clone();
                                        menu_open.set(false);
                                        spawn(async move {
                                            let ok = document::eval("return confirm('Board inkl. aller Karten löschen?');")
                                                .await
                                                .ok()
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(false);
                                            if ok && delete_board(id).await.is_ok() {
                                                navigator.push(Route::Boards {});
                                            }
                                        });
                                    },
                                    "Board löschen"
                                }
                            }
                        }
                    }
                }
            }

            if show_filter() {
                div { class: "flex flex-wrap items-center gap-2 px-4 pb-4 sm:px-8",
                    input {
                        class: "{INPUT} max-w-64",
                        placeholder: "Karten durchsuchen …",
                        value: search(),
                        oninput: move |e| search.set(e.value()),
                    }
                    for l in labels.iter().cloned() {
                        button {
                            key: "{l.id}",
                            class: if label_filter().as_deref() == Some(l.name.as_str()) { "rounded-full ring-1 ring-foreground/50" } else { "opacity-70 hover:opacity-100" },
                            onclick: {
                                let n = l.name.clone();
                                move |_| {
                                    if label_filter().as_deref() == Some(n.as_str()) {
                                        label_filter.set(None)
                                    } else {
                                        label_filter.set(Some(n.clone()))
                                    }
                                }
                            },
                            LabelBadge { name: l.name.clone(), color: l.color.clone() }
                        }
                    }
                    if label_filter().is_some() || !search().is_empty() {
                        button {
                            class: "inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground",
                            onclick: move |_| {
                                label_filter.set(None);
                                search.set(String::new());
                            },
                            X { class: "size-3" }
                            "Zurücksetzen"
                        }
                    }
                }
            }

            // Spalten
            div { class: "flex min-h-0 flex-1 snap-x snap-mandatory items-start gap-4 overflow-x-auto scroll-px-4 px-4 pb-6 sm:snap-none sm:px-8",
                for (ci, col) in columns.iter().cloned().enumerate() {
                    {
                        let col_cards: Vec<Card> = full.cards.iter().filter(|c| c.column_id == col.id).cloned().collect();
                        let visible: Vec<(usize, Card)> = col_cards.iter().cloned().enumerate().filter(|(_, c)| matches(c)).collect();
                        let count = col_cards.len();
                        let col_id = col.id.clone();
                        let col_id_drop = col.id.clone();
                        rsx! {
                            BoardColumn {
                                key: "{col.id}",
                                column: col.clone(),
                                count,
                                is_first: ci == 0,
                                is_last: ci + 1 == columns.len(),
                                index: ci,
                                drop_at_end: drop_target().is_some_and(|t| t.column_id == col.id && t.index >= count) && dragging().is_some(),
                                on_change: reload,
                                ondragover: move |_| {
                                    if dragging().is_some() {
                                        drop_target.set(Some(DropTarget { column_id: col_id.clone(), index: usize::MAX }));
                                    }
                                },
                                ondrop: move |_| {
                                    if let Some(card_id) = dragging() {
                                        do_move(card_id, col_id_drop.clone(), usize::MAX);
                                    }
                                    dragging.set(None);
                                    drop_target.set(None);
                                },
                                on_add: {
                                    let bid = board.id.clone();
                                    let cid = col.id.clone();
                                    move |text: String| {
                                        let bid = bid.clone();
                                        let input = card_from_text(&text, &cid);
                                        spawn(async move {
                                            let _ = create_card(bid, input).await;
                                            res.restart();
                                        });
                                    }
                                },
                                for (idx, card) in visible {
                                    {
                                        let cid = card.id.clone();
                                        let cid2 = card.id.clone();
                                        let colid = col.id.clone();
                                        let colid2 = col.id.clone();
                                        rsx! {
                                            BoardCard {
                                                key: "{card.id}",
                                                card: card.clone(),
                                                labels: labels.clone(),
                                                in_done_column: col.is_done,
                                                dragging: dragging().as_deref() == Some(card.id.as_str()),
                                                drop_before: dragging().is_some_and(|d| d != card.id) && drop_target() == Some(DropTarget { column_id: col.id.clone(), index: idx }),
                                                onopen: move |_| open_card.set(Some(cid.clone())),
                                                ondragstart: move |e: DragEvent| {
                                                    let _ = e.data_transfer().set_data("text/plain", &cid2);
                                                    e.data_transfer().set_effect_allowed("move");
                                                    dragging.set(Some(cid2.clone()));
                                                },
                                                ondragend: move |_| {
                                                    dragging.set(None);
                                                    drop_target.set(None);
                                                },
                                                ondragover: move |_| {
                                                    if dragging().is_some() {
                                                        drop_target.set(Some(DropTarget { column_id: colid.clone(), index: idx }));
                                                    }
                                                },
                                                ondrop: move |_| {
                                                    if let Some(card_id) = dragging() {
                                                        let ids = col_cards_ids(&full_cards_snapshot(local), &colid2);
                                                        let target = idx_after_removal(&ids, &card_id, idx);
                                                        do_move(card_id, colid2.clone(), target);
                                                    }
                                                    dragging.set(None);
                                                    drop_target.set(None);
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Neue Liste
                if adding_column() {
                    form {
                        class: "flex w-72 shrink-0 flex-col gap-2 rounded-xl border bg-card/40 p-3",
                        onsubmit: move |e| {
                            e.prevent_default();
                            let name = new_column().trim().to_string();
                            if name.is_empty() {
                                return;
                            }
                            let bid = bid_col.clone();
                            new_column.set(String::new());
                            spawn(async move {
                                let _ = create_column(bid, name).await;
                                res.restart();
                            });
                        },
                        input {
                            class: INPUT,
                            placeholder: "Name der Liste",
                            value: new_column(),
                            onmounted: move |e| async move { let _ = e.set_focus(true).await; },
                            oninput: move |e| new_column.set(e.value()),
                            onkeydown: move |e| if e.key() == Key::Escape { adding_column.set(false) },
                        }
                        div { class: "flex gap-2",
                            Button { size: ButtonSize::Sm, submit: true, "Hinzufügen" }
                            Button { size: ButtonSize::Sm, variant: ButtonVariant::Ghost, onclick: move |_| adding_column.set(false), "Abbrechen" }
                        }
                    }
                } else {
                    button {
                        class: "flex w-72 shrink-0 items-center gap-2 rounded-xl border border-dashed px-4 py-3 text-sm text-muted-foreground hover:border-solid hover:text-foreground",
                        onclick: move |_| adding_column.set(true),
                        Plus { class: "size-4" }
                        "Liste hinzufügen"
                    }
                }
            }
        }

        if show_activity() {
            ActivityPanel { board_id: board.id.clone(), version: version(), onclose: move |_| show_activity.set(false) }
        }
        if let Some(cid) = open_card() {
            CardDialog {
                key: "{cid}",
                card_id: cid,
                columns: columns.clone(),
                labels: labels.clone(),
                onclose: move |reload: bool| {
                    open_card.set(None);
                    if reload {
                        res.restart();
                    }
                },
            }
        }
    }
}

fn full_cards_snapshot(local: Signal<Option<BoardFull>>) -> Vec<Card> {
    local.peek().as_ref().map(|b| b.cards.clone()).unwrap_or_default()
}

fn col_cards_ids(cards: &[Card], column_id: &str) -> Vec<String> {
    let mut v: Vec<&Card> = cards.iter().filter(|c| c.column_id == column_id).collect();
    v.sort_by_key(|c| c.position);
    v.into_iter().map(|c| c.id.clone()).collect()
}

/// Ziel-Index, wenn die gezogene Karte vorher aus derselben Spalte entfernt wird.
fn idx_after_removal(column_ids: &[String], dragged: &str, target: usize) -> usize {
    match column_ids.iter().position(|id| id == dragged) {
        Some(from) if from < target => target - 1,
        _ => target,
    }
}

/// "Titel #label !! morgen" → Karte mit Labels, Priorität und Fälligkeit.
fn card_from_text(text: &str, column_id: &str) -> NewCard {
    let labels: Vec<String> = text
        .split_whitespace()
        .filter_map(|w| w.strip_prefix('#'))
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    let rest: String = text.split_whitespace().filter(|w| !w.starts_with('#')).collect::<Vec<_>>().join(" ");
    let q = parse_quick_add(&rest, today());
    NewCard {
        title: if q.title.is_empty() { rest } else { q.title },
        column: Some(column_id.to_string()),
        labels,
        priority: Some(q.priority),
        due_date: q.due,
        ..Default::default()
    }
}

#[component]
fn BoardColumn(
    column: Column,
    count: usize,
    index: usize,
    is_first: bool,
    is_last: bool,
    drop_at_end: bool,
    on_change: EventHandler<()>,
    on_add: EventHandler<String>,
    ondragover: EventHandler<()>,
    ondrop: EventHandler<()>,
    children: Element,
) -> Element {
    let mut composing = use_signal(|| false);
    let mut text = use_signal(String::new);
    let mut menu = use_signal(|| false);
    let mut renaming = use_signal(|| false);
    let mut name = use_signal(|| column.name.clone());

    let col_id = column.id.clone();
    let mut submit = move || {
        let t = text().trim().to_string();
        if !t.is_empty() {
            on_add.call(t);
            text.set(String::new());
        }
    };

    let menu_btn = "w-full rounded-md px-3 py-1.5 text-left hover:bg-accent";
    let cid_rename = col_id.clone();
    let cid_done = col_id.clone();
    let cid_left = col_id.clone();
    let cid_right = col_id.clone();
    let cid_del = col_id.clone();

    rsx! {
        div {
            class: "flex max-h-full w-[85vw] shrink-0 snap-start flex-col rounded-xl border bg-card/40 sm:w-72",
            ondragover: move |e| {
                e.prevent_default();
                ondragover.call(());
            },
            ondrop: move |e| {
                e.prevent_default();
                ondrop.call(());
            },
            div { class: "flex items-center gap-2 px-4 pb-2 pt-3",
                if renaming() {
                    input {
                        class: "{GHOST_INPUT} text-sm font-medium",
                        value: name(),
                        onmounted: move |e| async move { let _ = e.set_focus(true).await; },
                        oninput: move |e| name.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                let id = cid_rename.clone();
                                let n = name();
                                renaming.set(false);
                                spawn(async move {
                                    let _ = update_column(id, Some(n), None).await;
                                    on_change.call(());
                                });
                            } else if e.key() == Key::Escape {
                                renaming.set(false);
                            }
                        },
                        onblur: move |_| renaming.set(false),
                    }
                } else {
                    h3 {
                        class: "flex-1 truncate text-sm font-medium",
                        ondoubleclick: move |_| renaming.set(true),
                        "{column.name}"
                    }
                }
                span { class: "text-xs text-muted-foreground", "{count}" }
                button {
                    class: "rounded-md p-1 text-muted-foreground hover:bg-accent hover:text-foreground",
                    title: "Karte hinzufügen",
                    onclick: move |_| composing.set(true),
                    Plus { class: "size-4" }
                }
                div { class: "relative",
                    button {
                        class: "rounded-md p-1 text-muted-foreground hover:bg-accent hover:text-foreground",
                        onclick: move |_| menu.toggle(),
                        Ellipsis { class: "size-4" }
                    }
                    if menu() {
                        div { class: "fixed inset-0 z-40", onclick: move |_| menu.set(false) }
                        div { class: "absolute right-0 top-8 z-50 w-56 rounded-lg border bg-popover p-1 text-sm shadow-xl",
                            button { class: menu_btn, onclick: move |_| { menu.set(false); renaming.set(true); }, "Umbenennen" }
                            button {
                                class: menu_btn,
                                onclick: move |_| {
                                    menu.set(false);
                                    let id = cid_done.clone();
                                    let v = !column.is_done;
                                    spawn(async move {
                                        let _ = update_column(id, None, Some(v)).await;
                                        on_change.call(());
                                    });
                                },
                                if column.is_done { "Nicht mehr als erledigt zählen" } else { "Als „Erledigt“-Liste markieren" }
                            }
                            if !is_first {
                                button {
                                    class: menu_btn,
                                    onclick: move |_| {
                                        menu.set(false);
                                        let id = cid_left.clone();
                                        spawn(async move {
                                            let _ = move_column(id, index as i64 - 1).await;
                                            on_change.call(());
                                        });
                                    },
                                    "Nach links"
                                }
                            }
                            if !is_last {
                                button {
                                    class: menu_btn,
                                    onclick: move |_| {
                                        menu.set(false);
                                        let id = cid_right.clone();
                                        spawn(async move {
                                            let _ = move_column(id, index as i64 + 1).await;
                                            on_change.call(());
                                        });
                                    },
                                    "Nach rechts"
                                }
                            }
                            button {
                                class: "w-full rounded-md px-3 py-1.5 text-left text-destructive hover:bg-destructive/10",
                                onclick: move |_| {
                                    menu.set(false);
                                    let id = cid_del.clone();
                                    spawn(async move {
                                        let ok = document::eval("return confirm('Liste inkl. Karten löschen?');")
                                            .await
                                            .ok()
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false);
                                        if ok {
                                            let _ = delete_column(id).await;
                                            on_change.call(());
                                        }
                                    });
                                },
                                "Liste löschen"
                            }
                        }
                    }
                }
            }
            div { class: "flex min-h-12 flex-col gap-2.5 overflow-y-auto px-2 pb-2",
                {children}
                if drop_at_end {
                    div { class: "mx-1 h-0.5 rounded-full bg-foreground/60" }
                }
                if composing() {
                    div { class: "rounded-lg border bg-card p-2",
                        textarea {
                            class: "min-h-14 w-full resize-none bg-transparent text-sm outline-none placeholder:text-muted-foreground",
                            placeholder: "Titel … (#label, !!, morgen)",
                            value: text(),
                            onmounted: move |e| async move { let _ = e.set_focus(true).await; },
                            oninput: move |e| text.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter && !e.modifiers().shift() {
                                    e.prevent_default();
                                    submit();
                                } else if e.key() == Key::Escape {
                                    composing.set(false);
                                }
                            },
                        }
                        div { class: "flex gap-2",
                            Button { size: ButtonSize::Sm, onclick: move |_| submit(), "Hinzufügen" }
                            Button { size: ButtonSize::Sm, variant: ButtonVariant::Ghost, onclick: move |_| composing.set(false), "Fertig" }
                        }
                    }
                } else {
                    button {
                        class: "flex items-center gap-2 rounded-lg px-2 py-1.5 text-sm text-muted-foreground hover:bg-accent/40 hover:text-foreground",
                        onclick: move |_| composing.set(true),
                        Plus { class: "size-4" }
                        "Karte hinzufügen"
                    }
                }
            }
        }
    }
}
