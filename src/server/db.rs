//! Datenbankzugriff (SQLite). Wird von Server-Funktionen (UI) und REST-API gemeinsam genutzt.

use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use sqlx::SqlitePool;
use uuid::Uuid;

use serde_json::json;

use super::AppError;
use super::events::{Emit, emit};
use crate::model::*;

type Result<T> = std::result::Result<T, AppError>;

fn new_id() -> String {
    Uuid::now_v7().to_string()
}

fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

/// Einheitliches Zeitformat in der DB, damit String-Vergleiche (Bereichsabfragen) stimmen.
fn ts(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn day(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn clean(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn required(s: &str, field: &str) -> Result<String> {
    let s = s.trim();
    if s.is_empty() {
        return Err(AppError::BadRequest(format!("`{field}` darf nicht leer sein")));
    }
    Ok(s.to_string())
}

fn clean_labels(labels: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in labels {
        let l = l.trim().to_string();
        if !l.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&l)) {
            out.push(l);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Boards
// ---------------------------------------------------------------------------

pub async fn list_boards(pool: &SqlitePool) -> Result<Vec<BoardSummary>> {
    Ok(sqlx::query_as::<_, BoardSummary>(
        "SELECT b.id, b.name, b.description, b.position,
            (SELECT COUNT(*) FROM cards c WHERE c.board_id = b.id) AS card_count,
            (SELECT COUNT(*) FROM cards c JOIN board_columns col ON col.id = c.column_id
              WHERE c.board_id = b.id AND col.is_done = 0) AS open_count
         FROM boards b ORDER BY b.position, b.created_at",
    )
    .fetch_all(pool)
    .await?)
}

/// Board per ID oder Name (Gross-/Kleinschreibung egal).
pub async fn resolve_board(pool: &SqlitePool, key: &str) -> Result<Board> {
    let boards = sqlx::query_as::<_, Board>("SELECT id, name, description, position FROM boards").fetch_all(pool).await?;
    let key = key.trim();
    boards
        .iter()
        .find(|b| b.id == key)
        .or_else(|| boards.iter().find(|b| b.name.to_lowercase() == key.to_lowercase()))
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("Board `{key}` nicht gefunden")))
}

pub async fn create_board(pool: &SqlitePool, input: NewBoard) -> Result<Board> {
    let name = required(&input.name, "name")?;
    let id = new_id();
    let position: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(position), -1) + 1 FROM boards").fetch_one(pool).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO boards (id, name, description, position) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(&name)
        .bind(clean(input.description.clone()))
        .bind(position)
        .execute(&mut *tx)
        .await?;
    let columns = input
        .columns
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| ["Backlog", "Geplant", "In Arbeit", "Erledigt"].map(String::from).to_vec());
    for (i, col) in columns.iter().enumerate() {
        let lower = col.to_lowercase();
        let is_done = ["erledigt", "done", "fertig", "abgeschlossen"].contains(&lower.trim());
        sqlx::query("INSERT INTO board_columns (id, board_id, name, position, is_done) VALUES (?, ?, ?, ?, ?)")
            .bind(new_id())
            .bind(&id)
            .bind(col.trim())
            .bind(i as i64)
            .bind(is_done)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    resolve_board(pool, &id).await
}

pub async fn update_board(pool: &SqlitePool, id: &str, name: String, description: Option<String>) -> Result<Board> {
    let name = required(&name, "name")?;
    sqlx::query("UPDATE boards SET name = ?, description = ? WHERE id = ?")
        .bind(name)
        .bind(clean(description))
        .bind(id)
        .execute(pool)
        .await?;
    resolve_board(pool, id).await
}

pub async fn delete_board(pool: &SqlitePool, id: &str) -> Result<()> {
    let res = sqlx::query("DELETE FROM boards WHERE id = ?").bind(id).execute(pool).await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Board nicht gefunden".into()));
    }
    Ok(())
}

pub async fn get_board_full(pool: &SqlitePool, key: &str) -> Result<BoardFull> {
    let board = resolve_board(pool, key).await?;
    let columns = list_columns(pool, &board.id).await?;
    let labels = sqlx::query_as::<_, Label>("SELECT id, board_id, name, color FROM labels WHERE board_id = ? ORDER BY name")
        .bind(&board.id)
        .fetch_all(pool)
        .await?;
    let cards = sqlx::query_as::<_, Card>(&format!("{CARD_SELECT} WHERE c.board_id = ? ORDER BY c.position"))
        .bind(&board.id)
        .fetch_all(pool)
        .await?;
    Ok(BoardFull { board, columns, labels, cards })
}

// ---------------------------------------------------------------------------
// Spalten
// ---------------------------------------------------------------------------

async fn list_columns(pool: &SqlitePool, board_id: &str) -> Result<Vec<Column>> {
    Ok(sqlx::query_as::<_, Column>(
        "SELECT id, board_id, name, position, is_done FROM board_columns WHERE board_id = ? ORDER BY position",
    )
    .bind(board_id)
    .fetch_all(pool)
    .await?)
}

/// Spalte per ID oder Name innerhalb eines Boards.
pub async fn resolve_column(pool: &SqlitePool, board_id: &str, key: &str) -> Result<Column> {
    let cols = list_columns(pool, board_id).await?;
    let key = key.trim();
    cols.iter()
        .find(|c| c.id == key)
        .or_else(|| cols.iter().find(|c| c.name.to_lowercase() == key.to_lowercase()))
        .cloned()
        .ok_or_else(|| {
            let names: Vec<_> = cols.iter().map(|c| c.name.as_str()).collect();
            AppError::BadRequest(format!("Spalte `{key}` nicht gefunden (vorhanden: {})", names.join(", ")))
        })
}

pub async fn create_column(pool: &SqlitePool, board_id: &str, name: String) -> Result<Column> {
    let name = required(&name, "name")?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO board_columns (id, board_id, name, position)
         VALUES (?, ?, ?, (SELECT COALESCE(MAX(position), -1) + 1 FROM board_columns WHERE board_id = ?))",
    )
    .bind(&id)
    .bind(board_id)
    .bind(name)
    .bind(board_id)
    .execute(pool)
    .await?;
    resolve_column(pool, board_id, &id).await
}

pub async fn update_column(pool: &SqlitePool, id: &str, name: Option<String>, is_done: Option<bool>) -> Result<()> {
    if let Some(name) = name {
        let name = required(&name, "name")?;
        sqlx::query("UPDATE board_columns SET name = ? WHERE id = ?").bind(name).bind(id).execute(pool).await?;
    }
    if let Some(done) = is_done {
        sqlx::query("UPDATE board_columns SET is_done = ? WHERE id = ?").bind(done).bind(id).execute(pool).await?;
    }
    Ok(())
}

pub async fn delete_column(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM board_columns WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn move_column(pool: &SqlitePool, id: &str, index: i64) -> Result<()> {
    let board_id: String = sqlx::query_scalar("SELECT board_id FROM board_columns WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Spalte nicht gefunden".into()))?;
    let mut ids: Vec<String> = list_columns(pool, &board_id).await?.into_iter().map(|c| c.id).filter(|c| c != id).collect();
    let idx = (index.max(0) as usize).min(ids.len());
    ids.insert(idx, id.to_string());
    let mut tx = pool.begin().await?;
    for (i, cid) in ids.iter().enumerate() {
        sqlx::query("UPDATE board_columns SET position = ? WHERE id = ?").bind(i as i64).bind(cid).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

async fn ensure_labels(pool: &SqlitePool, board_id: &str, names: &[String]) -> Result<()> {
    for name in names {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM labels WHERE board_id = ? AND LOWER(name) = LOWER(?)")
                .bind(board_id)
                .bind(name)
                .fetch_optional(pool)
                .await?;
        if exists.is_none() {
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM labels WHERE board_id = ?").bind(board_id).fetch_one(pool).await?;
            let color = LABEL_COLORS[count as usize % LABEL_COLORS.len()];
            sqlx::query("INSERT OR IGNORE INTO labels (id, board_id, name, color) VALUES (?, ?, ?, ?)")
                .bind(new_id())
                .bind(board_id)
                .bind(name)
                .bind(color)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

pub async fn set_label_color(pool: &SqlitePool, id: &str, color: &str) -> Result<()> {
    if !LABEL_COLORS.contains(&color) {
        return Err(AppError::BadRequest(format!("Unbekannte Farbe `{color}`")));
    }
    sqlx::query("UPDATE labels SET color = ? WHERE id = ?").bind(color).bind(id).execute(pool).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Karten
// ---------------------------------------------------------------------------

const CARD_SELECT: &str = "SELECT c.id, c.board_id, c.column_id, c.title, c.description, c.labels, c.priority,
    c.due_date, c.url, c.source, c.external_id, c.position,
    (SELECT COUNT(*) FROM checklist_items i WHERE i.card_id = c.id) AS checklist_total,
    (SELECT COUNT(*) FROM checklist_items i WHERE i.card_id = c.id AND i.done = 1) AS checklist_done,
    c.created_at, c.updated_at
    FROM cards c";

const CARD_REF_SELECT: &str = "SELECT c.id, c.board_id, c.column_id, c.title, c.description, c.labels, c.priority,
    c.due_date, c.url, c.source, c.external_id, c.position,
    (SELECT COUNT(*) FROM checklist_items i WHERE i.card_id = c.id) AS checklist_total,
    (SELECT COUNT(*) FROM checklist_items i WHERE i.card_id = c.id AND i.done = 1) AS checklist_done,
    c.created_at, c.updated_at, b.name AS board_name, col.name AS column_name
    FROM cards c JOIN boards b ON b.id = c.board_id JOIN board_columns col ON col.id = c.column_id";

pub async fn get_card(pool: &SqlitePool, id: &str) -> Result<Card> {
    sqlx::query_as::<_, Card>(&format!("{CARD_SELECT} WHERE c.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Karte `{id}` nicht gefunden")))
}

pub async fn get_card_detail(pool: &SqlitePool, id: &str) -> Result<CardDetail> {
    let r = sqlx::query_as::<_, CardRef>(&format!("{CARD_REF_SELECT} WHERE c.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Karte `{id}` nicht gefunden")))?;
    let checklist = sqlx::query_as::<_, ChecklistItem>(
        "SELECT id, card_id, text, done, position FROM checklist_items WHERE card_id = ? ORDER BY position",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(CardDetail { card: r.card, column_name: r.column_name, board_name: r.board_name, checklist })
}

pub async fn find_card_external(pool: &SqlitePool, board_id: &str, source: &str, external_id: &str) -> Result<Card> {
    sqlx::query_as::<_, Card>(&format!("{CARD_SELECT} WHERE c.board_id = ? AND c.source = ? AND c.external_id = ?"))
        .bind(board_id)
        .bind(source)
        .bind(external_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Karte `{source}/{external_id}` nicht gefunden")))
}

#[derive(Default)]
pub struct CardFilter {
    pub column: Option<String>,
    pub label: Option<String>,
    pub source: Option<String>,
    pub external_id: Option<String>,
    pub q: Option<String>,
}

pub async fn list_cards(pool: &SqlitePool, board_key: &str, f: CardFilter) -> Result<Vec<Card>> {
    let full = get_board_full(pool, board_key).await?;
    let column_id = match &f.column {
        Some(c) => Some(resolve_column(pool, &full.board.id, c).await?.id),
        None => None,
    };
    let order: Vec<&str> = full.columns.iter().map(|c| c.id.as_str()).collect();
    let mut cards: Vec<Card> = full
        .cards
        .into_iter()
        .filter(|c| column_id.as_ref().is_none_or(|id| &c.column_id == id))
        .filter(|c| f.label.as_ref().is_none_or(|l| c.labels.iter().any(|x| x.eq_ignore_ascii_case(l))))
        .filter(|c| f.source.as_ref().is_none_or(|s| c.source.as_deref() == Some(s)))
        .filter(|c| f.external_id.as_ref().is_none_or(|s| c.external_id.as_deref() == Some(s)))
        .filter(|c| f.q.as_ref().is_none_or(|q| c.title.to_lowercase().contains(&q.to_lowercase())))
        .collect();
    cards.sort_by_key(|c| (order.iter().position(|o| *o == c.column_id), c.position));
    Ok(cards)
}

/// Legt eine Karte an. Existiert bereits eine Karte mit gleicher `source` + `external_id`,
/// wird diese aktualisiert (Upsert). Rückgabe: (Karte, neu angelegt?)
pub async fn create_card(pool: &SqlitePool, board_key: &str, input: NewCard) -> Result<(Card, bool)> {
    let board = resolve_board(pool, board_key).await?;
    let source = clean(input.source.clone());
    let external_id = clean(input.external_id.clone());

    if let (Some(s), Some(e)) = (&source, &external_id)
        && let Ok(existing) = find_card_external(pool, &board.id, s, e).await
    {
        let patch = CardPatch {
            title: Some(input.title),
            description: input.description.map(Some),
            column: input.column,
            labels: if input.labels.is_empty() { None } else { Some(input.labels) },
            priority: input.priority,
            due_date: input.due_date.map(Some),
            url: input.url.map(Some),
            position: None,
        };
        return Ok((update_card(pool, &existing.id, patch).await?, false));
    }

    let title = required(&input.title, "title")?;
    let column = match &input.column {
        Some(c) => resolve_column(pool, &board.id, c).await?,
        None => list_columns(pool, &board.id)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::BadRequest("Board hat keine Spalten".into()))?,
    };
    let labels = clean_labels(input.labels);
    ensure_labels(pool, &board.id, &labels).await?;
    let id = new_id();
    let now = ts(now());
    sqlx::query(
        "INSERT INTO cards (id, board_id, column_id, title, description, labels, priority, due_date, url, source,
            external_id, position, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
            (SELECT COALESCE(MAX(position), -1) + 1 FROM cards WHERE column_id = ?), ?, ?)",
    )
    .bind(&id)
    .bind(&board.id)
    .bind(&column.id)
    .bind(title)
    .bind(clean(input.description))
    .bind(serde_json::to_string(&labels).unwrap_or_else(|_| "[]".into()))
    .bind(input.priority.unwrap_or(0).clamp(0, 3))
    .bind(input.due_date.map(day))
    .bind(clean(input.url))
    .bind(source)
    .bind(external_id)
    .bind(&column.id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    for text in input.checklist {
        add_checklist_item(pool, &id, text).await?;
    }
    let card = get_card(pool, &id).await?;
    emit(
        pool,
        Emit {
            kind: "card.created",
            message: format!("„{}“ in {} angelegt", card.title, column.name),
            board_id: Some(&board.id),
            card_id: Some(&card.id),
            data: json!({ "card": card, "column": column.name, "board": board.name }),
        },
    )
    .await;
    Ok((card, true))
}

pub async fn update_card(pool: &SqlitePool, id: &str, p: CardPatch) -> Result<Card> {
    let card = get_card(pool, id).await?;
    let changed: Vec<&str> = [
        ("title", p.title.is_some()),
        ("description", p.description.is_some()),
        ("labels", p.labels.is_some()),
        ("priority", p.priority.is_some()),
        ("due_date", p.due_date.is_some()),
        ("url", p.url.is_some()),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.then_some(k))
    .collect();
    if let Some(title) = p.title {
        let title = required(&title, "title")?;
        sqlx::query("UPDATE cards SET title = ? WHERE id = ?").bind(title).bind(id).execute(pool).await?;
    }
    if let Some(desc) = p.description {
        sqlx::query("UPDATE cards SET description = ? WHERE id = ?").bind(clean(desc)).bind(id).execute(pool).await?;
    }
    if let Some(labels) = p.labels {
        let labels = clean_labels(labels);
        ensure_labels(pool, &card.board_id, &labels).await?;
        sqlx::query("UPDATE cards SET labels = ? WHERE id = ?")
            .bind(serde_json::to_string(&labels).unwrap_or_else(|_| "[]".into()))
            .bind(id)
            .execute(pool)
            .await?;
    }
    if let Some(prio) = p.priority {
        sqlx::query("UPDATE cards SET priority = ? WHERE id = ?").bind(prio.clamp(0, 3)).bind(id).execute(pool).await?;
    }
    if let Some(due) = p.due_date {
        sqlx::query("UPDATE cards SET due_date = ? WHERE id = ?").bind(due.map(day)).bind(id).execute(pool).await?;
    }
    if let Some(url) = p.url {
        sqlx::query("UPDATE cards SET url = ? WHERE id = ?").bind(clean(url)).bind(id).execute(pool).await?;
    }
    if p.column.is_some() || p.position.is_some() {
        let column_id = match &p.column {
            Some(c) => resolve_column(pool, &card.board_id, c).await?.id,
            None => card.column_id.clone(),
        };
        let position = match p.position {
            Some(pos) => pos,
            // Spaltenwechsel ohne Position: ans Ende
            None if column_id != card.column_id => i64::MAX,
            None => card.position,
        };
        if column_id != card.column_id || p.position.is_some() {
            move_card(pool, id, &column_id, position).await?;
        }
    }
    sqlx::query("UPDATE cards SET updated_at = ? WHERE id = ?").bind(ts(now())).bind(id).execute(pool).await?;
    let updated = get_card(pool, id).await?;
    // Nur melden, wenn sich wirklich etwas geändert hat (Spaltenwechsel meldet move_card).
    let differs = updated.title != card.title
        || updated.description != card.description
        || updated.labels != card.labels
        || updated.priority != card.priority
        || updated.due_date != card.due_date
        || updated.url != card.url;
    if !changed.is_empty() && differs {
        emit(
            pool,
            Emit {
                kind: "card.updated",
                message: format!("„{}“ bearbeitet", updated.title),
                board_id: Some(&updated.board_id),
                card_id: Some(&updated.id),
                data: json!({ "card": updated, "fields": changed }),
            },
        )
        .await;
    }
    Ok(updated)
}

pub async fn move_card(pool: &SqlitePool, id: &str, column_id: &str, index: i64) -> Result<()> {
    let card = get_card(pool, id).await?;
    let target_board: Option<String> = sqlx::query_scalar("SELECT board_id FROM board_columns WHERE id = ?")
        .bind(column_id)
        .fetch_optional(pool)
        .await?;
    if target_board.as_deref() != Some(card.board_id.as_str()) {
        return Err(AppError::BadRequest("Spalte gehört nicht zum Board der Karte".into()));
    }
    let mut ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM cards WHERE column_id = ? AND id != ? ORDER BY position")
            .bind(column_id)
            .bind(id)
            .fetch_all(pool)
            .await?;
    let idx = (index.max(0) as usize).min(ids.len());
    ids.insert(idx, id.to_string());
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE cards SET column_id = ?, updated_at = ? WHERE id = ?")
        .bind(column_id)
        .bind(ts(now()))
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for (i, cid) in ids.iter().enumerate() {
        sqlx::query("UPDATE cards SET position = ? WHERE id = ?").bind(i as i64).bind(cid).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    if card.column_id != column_id {
        let name = |cid: String| async move {
            sqlx::query_scalar::<_, String>("SELECT name FROM board_columns WHERE id = ?")
                .bind(cid)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .unwrap_or_default()
        };
        let from = name(card.column_id.clone()).await;
        let to = name(column_id.to_string()).await;
        let moved = get_card(pool, id).await?;
        emit(
            pool,
            Emit {
                kind: "card.moved",
                message: format!("„{}“: {from} → {to}", moved.title),
                board_id: Some(&moved.board_id),
                card_id: Some(&moved.id),
                data: json!({ "card": moved, "from": from, "to": to }),
            },
        )
        .await;
    }
    Ok(())
}

pub async fn delete_card(pool: &SqlitePool, id: &str) -> Result<()> {
    let card = get_card(pool, id).await?;
    sqlx::query("DELETE FROM cards WHERE id = ?").bind(id).execute(pool).await?;
    emit(
        pool,
        Emit {
            kind: "card.deleted",
            message: format!("„{}“ gelöscht", card.title),
            board_id: Some(&card.board_id),
            card_id: None,
            data: json!({ "card": card }),
        },
    )
    .await;
    Ok(())
}

pub async fn add_checklist_item(pool: &SqlitePool, card_id: &str, text: String) -> Result<ChecklistItem> {
    let text = required(&text, "text")?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO checklist_items (id, card_id, text, position)
         VALUES (?, ?, ?, (SELECT COALESCE(MAX(position), -1) + 1 FROM checklist_items WHERE card_id = ?))",
    )
    .bind(&id)
    .bind(card_id)
    .bind(text)
    .bind(card_id)
    .execute(pool)
    .await?;
    Ok(sqlx::query_as::<_, ChecklistItem>("SELECT id, card_id, text, done, position FROM checklist_items WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?)
}

pub async fn set_checklist_item(pool: &SqlitePool, id: &str, done: bool) -> Result<()> {
    sqlx::query("UPDATE checklist_items SET done = ? WHERE id = ?").bind(done).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn delete_checklist_item(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM checklist_items WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Todos
// ---------------------------------------------------------------------------

const TODO_SELECT: &str = "SELECT id, title, notes, list, due_date, priority, done, done_at, source, external_id, created_at, recurrence FROM todos";

fn clean_recurrence(r: Option<String>) -> Result<Option<String>> {
    match clean(r) {
        None => Ok(None),
        Some(r) if RECURRENCES.iter().any(|(k, _)| *k == r) => Ok(Some(r)),
        Some(r) => Err(AppError::BadRequest(format!(
            "Unbekannte Wiederholung `{r}` (erlaubt: daily, weekdays, weekly, monthly, yearly)"
        ))),
    }
}

/// Nächster Termin einer Wiederholung (immer nach `from`).
pub fn next_occurrence(rule: &str, from: NaiveDate) -> NaiveDate {
    use chrono::{Datelike, Months, Weekday};
    match rule {
        "weekdays" => {
            let mut d = from + Duration::days(1);
            while matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
                d += Duration::days(1);
            }
            d
        }
        "weekly" => from + Duration::days(7),
        "monthly" => from.checked_add_months(Months::new(1)).unwrap_or(from + Duration::days(30)),
        "yearly" => from.checked_add_months(Months::new(12)).unwrap_or(from + Duration::days(365)),
        _ => from + Duration::days(1),
    }
}
const TODO_ORDER: &str = "ORDER BY done, due_date IS NULL, due_date, priority DESC, created_at";

#[derive(Default)]
pub struct TodoFilter {
    pub done: Option<bool>,
    pub list: Option<String>,
    pub due_before: Option<NaiveDate>,
    pub source: Option<String>,
    pub external_id: Option<String>,
}

pub async fn list_todos(pool: &SqlitePool, f: TodoFilter) -> Result<Vec<Todo>> {
    let all = sqlx::query_as::<_, Todo>(&format!("{TODO_SELECT} {TODO_ORDER}")).fetch_all(pool).await?;
    Ok(all
        .into_iter()
        .filter(|t| f.done.is_none_or(|d| t.done == d))
        .filter(|t| f.list.as_ref().is_none_or(|l| t.list.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(l))))
        .filter(|t| f.due_before.is_none_or(|d| t.due_date.is_some_and(|x| x <= d)))
        .filter(|t| f.source.as_ref().is_none_or(|s| t.source.as_deref() == Some(s)))
        .filter(|t| f.external_id.as_ref().is_none_or(|s| t.external_id.as_deref() == Some(s)))
        .collect())
}

/// Offene Todos + in den letzten 14 Tagen erledigte (für die UI).
pub async fn list_todos_ui(pool: &SqlitePool) -> Result<Vec<Todo>> {
    let since = ts(now() - Duration::days(14));
    Ok(sqlx::query_as::<_, Todo>(&format!("{TODO_SELECT} WHERE done = 0 OR done_at >= ? {TODO_ORDER}"))
        .bind(since)
        .fetch_all(pool)
        .await?)
}

pub async fn get_todo(pool: &SqlitePool, id: &str) -> Result<Todo> {
    sqlx::query_as::<_, Todo>(&format!("{TODO_SELECT} WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Todo `{id}` nicht gefunden")))
}

pub async fn create_todo(pool: &SqlitePool, input: NewTodo) -> Result<(Todo, bool)> {
    let source = clean(input.source.clone());
    let external_id = clean(input.external_id.clone());
    if let (Some(s), Some(e)) = (&source, &external_id) {
        let existing: Option<String> = sqlx::query_scalar("SELECT id FROM todos WHERE source = ? AND external_id = ?")
            .bind(s)
            .bind(e)
            .fetch_optional(pool)
            .await?;
        if let Some(id) = existing {
            let patch = TodoPatch {
                title: Some(input.title),
                notes: input.notes.map(Some),
                list: input.list.map(Some),
                due_date: input.due_date.map(Some),
                priority: input.priority,
                done: None,
                recurrence: input.recurrence.map(Some),
            };
            return Ok((update_todo(pool, &id, patch).await?, false));
        }
    }
    let title = required(&input.title, "title")?;
    let recurrence = clean_recurrence(input.recurrence)?;
    // Wiederkehrend ohne Datum: ab heute
    let due = input.due_date.or_else(|| recurrence.as_ref().map(|_| Local::now().date_naive()));
    let id = new_id();
    sqlx::query(
        "INSERT INTO todos (id, title, notes, list, due_date, priority, source, external_id, created_at, recurrence)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(title)
    .bind(clean(input.notes))
    .bind(clean(input.list))
    .bind(due.map(day))
    .bind(input.priority.unwrap_or(0).clamp(0, 3))
    .bind(source)
    .bind(external_id)
    .bind(ts(now()))
    .bind(recurrence)
    .execute(pool)
    .await?;
    let todo = get_todo(pool, &id).await?;
    emit(
        pool,
        Emit {
            kind: "todo.created",
            message: format!("Todo „{}“ angelegt", todo.title),
            board_id: None,
            card_id: None,
            data: json!({ "todo": todo }),
        },
    )
    .await;
    Ok((todo, true))
}

pub async fn update_todo(pool: &SqlitePool, id: &str, p: TodoPatch) -> Result<Todo> {
    let before = get_todo(pool, id).await?;
    if let Some(r) = p.recurrence.clone() {
        let r = clean_recurrence(r)?;
        sqlx::query("UPDATE todos SET recurrence = ? WHERE id = ?").bind(r).bind(id).execute(pool).await?;
    }
    if let Some(title) = p.title {
        let title = required(&title, "title")?;
        sqlx::query("UPDATE todos SET title = ? WHERE id = ?").bind(title).bind(id).execute(pool).await?;
    }
    if let Some(notes) = p.notes {
        sqlx::query("UPDATE todos SET notes = ? WHERE id = ?").bind(clean(notes)).bind(id).execute(pool).await?;
    }
    if let Some(list) = p.list {
        sqlx::query("UPDATE todos SET list = ? WHERE id = ?").bind(clean(list)).bind(id).execute(pool).await?;
    }
    if let Some(due) = p.due_date {
        sqlx::query("UPDATE todos SET due_date = ? WHERE id = ?").bind(due.map(day)).bind(id).execute(pool).await?;
    }
    if let Some(prio) = p.priority {
        sqlx::query("UPDATE todos SET priority = ? WHERE id = ?").bind(prio.clamp(0, 3)).bind(id).execute(pool).await?;
    }
    if let Some(done) = p.done {
        sqlx::query("UPDATE todos SET done = ?, done_at = ? WHERE id = ?")
            .bind(done)
            .bind(done.then(|| ts(now())))
            .bind(id)
            .execute(pool)
            .await?;
    }
    let todo = get_todo(pool, id).await?;
    if todo.done && !before.done {
        emit(
            pool,
            Emit {
                kind: "todo.completed",
                message: format!("Todo „{}“ erledigt", todo.title),
                board_id: None,
                card_id: None,
                data: json!({ "todo": todo }),
            },
        )
        .await;
        // Wiederkehrend: nächste Instanz anlegen
        if let Some(rule) = todo.recurrence.clone() {
            let base = todo.due_date.unwrap_or_else(|| Local::now().date_naive());
            let mut next = next_occurrence(&rule, base);
            let today = Local::now().date_naive();
            while next <= today && base < today {
                next = next_occurrence(&rule, next);
            }
            let input = NewTodo {
                title: todo.title.clone(),
                notes: todo.notes.clone(),
                list: todo.list.clone(),
                due_date: Some(next),
                priority: Some(todo.priority),
                recurrence: Some(rule),
                ..Default::default()
            };
            Box::pin(create_todo(pool, input)).await?;
            // Die erledigte Instanz wiederholt sich nicht mehr selbst
            sqlx::query("UPDATE todos SET recurrence = NULL WHERE id = ?").bind(id).execute(pool).await?;
        }
    } else if before != todo {
        emit(
            pool,
            Emit {
                kind: "todo.updated",
                message: format!("Todo „{}“ bearbeitet", todo.title),
                board_id: None,
                card_id: None,
                data: json!({ "todo": todo }),
            },
        )
        .await;
    }
    get_todo(pool, id).await
}

pub async fn delete_todo(pool: &SqlitePool, id: &str) -> Result<()> {
    let todo = get_todo(pool, id).await?;
    sqlx::query("DELETE FROM todos WHERE id = ?").bind(id).execute(pool).await?;
    emit(
        pool,
        Emit {
            kind: "todo.deleted",
            message: format!("Todo „{}“ gelöscht", todo.title),
            board_id: None,
            card_id: None,
            data: json!({ "todo": todo }),
        },
    )
    .await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Termine
// ---------------------------------------------------------------------------

const EVENT_SELECT: &str =
    "SELECT id, title, description, location, start_at, end_at, all_day, color, source, external_id FROM events";

/// Termine, die den Zeitraum [from, to) berühren.
pub async fn list_events(pool: &SqlitePool, from: NaiveDateTime, to: NaiveDateTime) -> Result<Vec<Event>> {
    Ok(sqlx::query_as::<_, Event>(&format!(
        "{EVENT_SELECT} WHERE start_at < ? AND end_at >= ? ORDER BY all_day DESC, start_at"
    ))
    .bind(ts(to))
    .bind(ts(from))
    .fetch_all(pool)
    .await?)
}

pub async fn get_event(pool: &SqlitePool, id: &str) -> Result<Event> {
    sqlx::query_as::<_, Event>(&format!("{EVENT_SELECT} WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Termin `{id}` nicht gefunden")))
}

fn event_color(c: Option<String>) -> String {
    c.filter(|c| EVENT_COLORS.contains(&c.as_str())).unwrap_or_else(|| "blue".into())
}

fn normalize_range(start: NaiveDateTime, end: NaiveDateTime, all_day: bool) -> (NaiveDateTime, NaiveDateTime) {
    if all_day {
        let s = start.date().and_hms_opt(0, 0, 0).unwrap_or(start);
        let e = end.date().max(start.date()).and_hms_opt(23, 59, 59).unwrap_or(end);
        (s, e)
    } else if end <= start {
        (start, start + Duration::hours(1))
    } else {
        (start, end)
    }
}

pub async fn create_event(pool: &SqlitePool, input: NewEvent) -> Result<(Event, bool)> {
    let source = clean(input.source.clone());
    let external_id = clean(input.external_id.clone());
    if let (Some(s), Some(e)) = (&source, &external_id) {
        let existing: Option<String> = sqlx::query_scalar("SELECT id FROM events WHERE source = ? AND external_id = ?")
            .bind(s)
            .bind(e)
            .fetch_optional(pool)
            .await?;
        if let Some(id) = existing {
            let patch = EventPatch {
                title: Some(input.title),
                description: input.description.map(Some),
                location: input.location.map(Some),
                start_at: Some(input.start_at),
                end_at: input.end_at,
                all_day: Some(input.all_day),
                color: input.color,
            };
            return Ok((update_event(pool, &id, patch).await?, false));
        }
    }
    let title = required(&input.title, "title")?;
    let end = input.end_at.unwrap_or(input.start_at + Duration::hours(1));
    let (start, end) = normalize_range(input.start_at, end, input.all_day);
    let id = new_id();
    sqlx::query(
        "INSERT INTO events (id, title, description, location, start_at, end_at, all_day, color, source, external_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(title)
    .bind(clean(input.description))
    .bind(clean(input.location))
    .bind(ts(start))
    .bind(ts(end))
    .bind(input.all_day)
    .bind(event_color(input.color))
    .bind(source)
    .bind(external_id)
    .execute(pool)
    .await?;
    let ev = get_event(pool, &id).await?;
    emit(
        pool,
        Emit {
            kind: "event.created",
            message: format!("Termin „{}“ am {} angelegt", ev.title, ev.start_at.format("%d.%m.%Y")),
            board_id: None,
            card_id: None,
            data: json!({ "event": ev }),
        },
    )
    .await;
    Ok((ev, true))
}

pub async fn update_event(pool: &SqlitePool, id: &str, p: EventPatch) -> Result<Event> {
    let ev = get_event(pool, id).await?;
    let title = match p.title {
        Some(t) => required(&t, "title")?,
        None => ev.title,
    };
    let description = p.description.map(clean).unwrap_or(ev.description);
    let location = p.location.map(clean).unwrap_or(ev.location);
    let all_day = p.all_day.unwrap_or(ev.all_day);
    let start = p.start_at.unwrap_or(ev.start_at);
    // Start verschoben, Ende nicht angegeben: Dauer beibehalten
    let end = p.end_at.unwrap_or(ev.end_at + (start - ev.start_at));
    let (start, end) = normalize_range(start, end, all_day);
    let color = p.color.map(|c| event_color(Some(c))).unwrap_or(ev.color);
    sqlx::query(
        "UPDATE events SET title = ?, description = ?, location = ?, start_at = ?, end_at = ?, all_day = ?, color = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(description)
    .bind(location)
    .bind(ts(start))
    .bind(ts(end))
    .bind(all_day)
    .bind(color)
    .bind(id)
    .execute(pool)
    .await?;
    let ev = get_event(pool, id).await?;
    emit(
        pool,
        Emit {
            kind: "event.updated",
            message: format!("Termin „{}“ bearbeitet", ev.title),
            board_id: None,
            card_id: None,
            data: json!({ "event": ev }),
        },
    )
    .await;
    Ok(ev)
}

pub async fn delete_event(pool: &SqlitePool, id: &str) -> Result<()> {
    let ev = get_event(pool, id).await?;
    sqlx::query("DELETE FROM events WHERE id = ?").bind(id).execute(pool).await?;
    emit(
        pool,
        Emit {
            kind: "event.deleted",
            message: format!("Termin „{}“ gelöscht", ev.title),
            board_id: None,
            card_id: None,
            data: json!({ "event": ev }),
        },
    )
    .await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Kalender & Heute
// ---------------------------------------------------------------------------

async fn cards_due_between(pool: &SqlitePool, from: NaiveDate, to: NaiveDate, open_only: bool) -> Result<Vec<CardRef>> {
    let extra = if open_only { "AND col.is_done = 0" } else { "" };
    Ok(sqlx::query_as::<_, CardRef>(&format!(
        "{CARD_REF_SELECT} WHERE c.due_date IS NOT NULL AND c.due_date >= ? AND c.due_date <= ? {extra}
         ORDER BY c.due_date, c.priority DESC"
    ))
    .bind(day(from))
    .bind(day(to))
    .fetch_all(pool)
    .await?)
}

/// Alles mit Datum im Bereich [from, to] (inklusive).
pub async fn calendar(pool: &SqlitePool, from: NaiveDate, to: NaiveDate) -> Result<CalendarData> {
    let start = from.and_hms_opt(0, 0, 0).unwrap_or_default();
    let end = (to + Duration::days(1)).and_hms_opt(0, 0, 0).unwrap_or_default();
    let events = list_events(pool, start, end).await?;
    let todos = sqlx::query_as::<_, Todo>(&format!(
        "{TODO_SELECT} WHERE due_date IS NOT NULL AND due_date >= ? AND due_date <= ? {TODO_ORDER}"
    ))
    .bind(day(from))
    .bind(day(to))
    .fetch_all(pool)
    .await?;
    let cards = cards_due_between(pool, from, to, false).await?;
    Ok(CalendarData { events, todos, cards })
}

pub async fn today(pool: &SqlitePool, date: NaiveDate) -> Result<TodayData> {
    let start = date.and_hms_opt(0, 0, 0).unwrap_or_default();
    let events = list_events(pool, start, start + Duration::days(1)).await?;
    let open = sqlx::query_as::<_, Todo>(&format!("{TODO_SELECT} WHERE done = 0 {TODO_ORDER}")).fetch_all(pool).await?;
    let open_total = open.len() as i64;
    let mut overdue = Vec::new();
    let mut today = Vec::new();
    let mut upcoming = Vec::new();
    for t in open {
        match t.due_date {
            Some(d) if d < date => overdue.push(t),
            Some(d) if d == date => today.push(t),
            Some(d) if d <= date + Duration::days(7) => upcoming.push(t),
            _ => {}
        }
    }
    let done_today: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM todos WHERE done = 1 AND done_at >= ?")
        .bind(ts(start))
        .fetch_one(pool)
        .await?;
    let cards_due =
        cards_due_between(pool, NaiveDate::from_ymd_opt(1970, 1, 1).unwrap_or(date), date + Duration::days(7), true)
            .await?;
    Ok(TodayData { date, events, overdue, today, upcoming, done_today, open_total, cards_due })
}

// ---------------------------------------------------------------------------
// API-Keys
// ---------------------------------------------------------------------------

pub fn hash_key(secret: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(secret.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

pub async fn list_api_keys(pool: &SqlitePool) -> Result<Vec<ApiKey>> {
    Ok(sqlx::query_as::<_, ApiKey>(
        "SELECT id, name, prefix, created_at, last_used_at FROM api_keys ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn create_api_key(pool: &SqlitePool, name: String) -> Result<CreatedApiKey> {
    let name = required(&name, "name")?;
    let secret = format!("nrzp_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let prefix: String = secret.chars().take(12).collect();
    let id = new_id();
    sqlx::query("INSERT INTO api_keys (id, name, prefix, key_hash, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(name)
        .bind(&prefix)
        .bind(hash_key(&secret))
        .bind(ts(now()))
        .execute(pool)
        .await?;
    let key = sqlx::query_as::<_, ApiKey>("SELECT id, name, prefix, created_at, last_used_at FROM api_keys WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?;
    Ok(CreatedApiKey { key, secret })
}

pub async fn delete_api_key(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM api_keys WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

/// Prüft einen API-Key, merkt sich die letzte Nutzung und liefert den Key-Namen.
pub async fn verify_api_key(pool: &SqlitePool, secret: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("UPDATE api_keys SET last_used_at = ? WHERE key_hash = ? RETURNING name")
        .bind(ts(now()))
        .bind(hash_key(secret))
        .fetch_optional(pool)
        .await?)
}

// ---------------------------------------------------------------------------
// Aktivität
// ---------------------------------------------------------------------------

pub async fn list_activity(pool: &SqlitePool, board_id: Option<&str>, limit: i64) -> Result<Vec<Activity>> {
    let sql = "SELECT id, kind, message, actor, board_id, card_id, created_at FROM activity";
    let limit = limit.clamp(1, 500);
    Ok(match board_id {
        Some(b) => {
            sqlx::query_as::<_, Activity>(&format!("{sql} WHERE board_id = ? ORDER BY created_at DESC, id DESC LIMIT ?"))
                .bind(b)
                .bind(limit)
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query_as::<_, Activity>(&format!("{sql} ORDER BY created_at DESC, id DESC LIMIT ?"))
                .bind(limit)
                .fetch_all(pool)
                .await?
        }
    })
}

// ---------------------------------------------------------------------------
// Webhooks
// ---------------------------------------------------------------------------

const WEBHOOK_SELECT: &str =
    "SELECT id, name, url, secret, events, active, created_at, last_status, last_error, last_at FROM webhooks";

pub async fn list_webhooks(pool: &SqlitePool) -> Result<Vec<Webhook>> {
    Ok(sqlx::query_as::<_, Webhook>(&format!("{WEBHOOK_SELECT} ORDER BY created_at")).fetch_all(pool).await?)
}

pub async fn get_webhook(pool: &SqlitePool, id: &str) -> Result<Webhook> {
    sqlx::query_as::<_, Webhook>(&format!("{WEBHOOK_SELECT} WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Webhook nicht gefunden".into()))
}

pub async fn create_webhook(pool: &SqlitePool, input: NewWebhook) -> Result<Webhook> {
    let name = required(&input.name, "name")?;
    let url = required(&input.url, "url")?;
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::BadRequest("URL muss mit http:// oder https:// beginnen".into()));
    }
    let id = new_id();
    sqlx::query("INSERT INTO webhooks (id, name, url, secret, events, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(name)
        .bind(url)
        .bind(clean(input.secret))
        .bind(serde_json::to_string(&input.events).unwrap_or_else(|_| "[]".into()))
        .bind(ts(now()))
        .execute(pool)
        .await?;
    get_webhook(pool, &id).await
}

pub async fn set_webhook_active(pool: &SqlitePool, id: &str, active: bool) -> Result<()> {
    sqlx::query("UPDATE webhooks SET active = ? WHERE id = ?").bind(active).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn delete_webhook(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM webhooks WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Einstellungen
// ---------------------------------------------------------------------------

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT value FROM settings WHERE key = ?").bind(key).fetch_optional(pool).await?)
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

/// Token für das Kalender-Abo (ICS). `rotate` erzeugt ein neues.
pub async fn ics_token(pool: &SqlitePool, rotate: bool) -> Result<String> {
    if !rotate && let Some(t) = get_setting(pool, "ics_token").await? {
        return Ok(t);
    }
    let token = Uuid::new_v4().simple().to_string();
    set_setting(pool, "ics_token", &token).await?;
    Ok(token)
}

// ---------------------------------------------------------------------------
// Suche
// ---------------------------------------------------------------------------

pub async fn search(pool: &SqlitePool, q: &str) -> Result<SearchResults> {
    let q = q.trim();
    if q.is_empty() {
        return Ok(SearchResults::default());
    }
    let like = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
    let boards = sqlx::query_as::<_, Board>(
        "SELECT id, name, description, position FROM boards WHERE name LIKE ? ESCAPE '\\' ORDER BY position LIMIT 5",
    )
    .bind(&like)
    .fetch_all(pool)
    .await?;
    let cards = sqlx::query_as::<_, CardRef>(&format!(
        "{CARD_REF_SELECT} WHERE c.title LIKE ?1 ESCAPE '\\' OR c.description LIKE ?1 ESCAPE '\\'
         OR c.external_id = ?2 ORDER BY col.is_done, c.updated_at DESC LIMIT 8"
    ))
    .bind(&like)
    .bind(q)
    .fetch_all(pool)
    .await?;
    let todos = sqlx::query_as::<_, Todo>(&format!(
        "{TODO_SELECT} WHERE title LIKE ?1 ESCAPE '\\' OR notes LIKE ?1 ESCAPE '\\' {TODO_ORDER} LIMIT 8"
    ))
    .bind(&like)
    .fetch_all(pool)
    .await?;
    let events = sqlx::query_as::<_, Event>(&format!(
        "{EVENT_SELECT} WHERE title LIKE ?1 ESCAPE '\\' OR location LIKE ?1 ESCAPE '\\'
         ORDER BY ABS(julianday(start_at) - julianday('now')) LIMIT 6"
    ))
    .bind(&like)
    .fetch_all(pool)
    .await?;
    Ok(SearchResults { boards, cards, todos, events })
}
