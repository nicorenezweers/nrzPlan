//! Gemeinsame Datentypen für UI, Server-Funktionen und REST-API.

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Deserializer, Serialize};

/// Unterscheidet bei PATCH zwischen "Feld fehlt" (`None`), "Feld = null" (`Some(None)`) und Wert.
fn double_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[allow(dead_code)]
pub const LABEL_COLORS: [&str; 9] = ["blue", "green", "orange", "red", "purple", "yellow", "pink", "teal", "gray"];
pub const EVENT_COLORS: [&str; 8] = ["blue", "green", "orange", "red", "purple", "yellow", "pink", "teal"];

// ---------------------------------------------------------------------------
// Boards
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Board {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct BoardSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub position: i64,
    pub card_count: i64,
    pub open_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Column {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub position: i64,
    pub is_done: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Label {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Card {
    pub id: String,
    pub board_id: String,
    pub column_id: String,
    pub title: String,
    pub description: Option<String>,
    #[cfg_attr(feature = "server", sqlx(json))]
    pub labels: Vec<String>,
    pub priority: i64,
    pub due_date: Option<NaiveDate>,
    pub url: Option<String>,
    pub source: Option<String>,
    pub external_id: Option<String>,
    pub position: i64,
    pub checklist_total: i64,
    pub checklist_done: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct ChecklistItem {
    pub id: String,
    pub card_id: String,
    pub text: String,
    pub done: bool,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardFull {
    pub board: Board,
    pub columns: Vec<Column>,
    pub labels: Vec<Label>,
    pub cards: Vec<Card>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardDetail {
    #[serde(flatten)]
    pub card: Card,
    pub column_name: String,
    pub board_name: String,
    pub checklist: Vec<ChecklistItem>,
}

/// Karte mit Board-Kontext (für Heute- und Kalenderansicht).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct CardRef {
    #[serde(flatten)]
    #[cfg_attr(feature = "server", sqlx(flatten))]
    pub card: Card,
    pub board_name: String,
    pub column_name: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewBoard {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Spaltennamen; Standard: Backlog, Geplant, In Arbeit, Erledigt.
    #[serde(default)]
    pub columns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewCard {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Spalte als ID oder Name (Gross-/Kleinschreibung egal). Alias: `status`.
    #[serde(default, alias = "status", alias = "list")]
    pub column: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub external_id: Option<String>,
    #[serde(default)]
    pub checklist: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CardPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    /// Spalte als ID oder Name. Alias: `status`.
    #[serde(default, alias = "status", alias = "list", skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
    pub due_date: Option<Option<NaiveDate>>,
    #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
    pub url: Option<Option<String>>,
    /// Neue Position (0-basiert) innerhalb der (neuen) Spalte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
}

// ---------------------------------------------------------------------------
// Todos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    pub list: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub priority: i64,
    pub done: bool,
    pub done_at: Option<NaiveDateTime>,
    pub source: Option<String>,
    pub external_id: Option<String>,
    pub created_at: NaiveDateTime,
    /// daily | weekdays | weekly | monthly | yearly
    pub recurrence: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewTodo {
    pub title: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub list: Option<String>,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub external_id: Option<String>,
    /// daily | weekdays | weekly | monthly | yearly
    #[serde(default)]
    pub recurrence: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TodoPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
    pub notes: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
    pub list: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
    pub due_date: Option<Option<NaiveDate>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,
    #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<Option<String>>,
}

pub const RECURRENCES: [(&str, &str); 5] = [
    ("daily", "Täglich"),
    ("weekdays", "Werktags"),
    ("weekly", "Wöchentlich"),
    ("monthly", "Monatlich"),
    ("yearly", "Jährlich"),
];

pub fn recurrence_label(r: &str) -> &'static str {
    RECURRENCES.iter().find(|(k, _)| *k == r).map(|(_, l)| *l).unwrap_or("Wiederholt")
}

// ---------------------------------------------------------------------------
// Kalender
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Event {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: NaiveDateTime,
    pub end_at: NaiveDateTime,
    pub all_day: bool,
    pub color: String,
    pub source: Option<String>,
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewEvent {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    pub start_at: NaiveDateTime,
    /// Standard: start + 1 Stunde (bzw. gleicher Tag bei ganztägig).
    #[serde(default)]
    pub end_at: Option<NaiveDateTime>,
    #[serde(default)]
    pub all_day: bool,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EventPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option", skip_serializing_if = "Option::is_none")]
    pub location: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_at: Option<NaiveDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_at: Option<NaiveDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all_day: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarData {
    pub events: Vec<Event>,
    pub todos: Vec<Todo>,
    pub cards: Vec<CardRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodayData {
    pub date: NaiveDate,
    pub events: Vec<Event>,
    pub overdue: Vec<Todo>,
    pub today: Vec<Todo>,
    pub upcoming: Vec<Todo>,
    pub done_today: i64,
    pub open_total: i64,
    pub cards_due: Vec<CardRef>,
}

// ---------------------------------------------------------------------------
// API-Keys
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub created_at: NaiveDateTime,
    pub last_used_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatedApiKey {
    pub key: ApiKey,
    /// Nur einmal sichtbar.
    pub secret: String,
}

// ---------------------------------------------------------------------------
// Aktivität, Webhooks, Suche
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Activity {
    pub id: String,
    pub kind: String,
    pub message: String,
    pub actor: String,
    pub board_id: Option<String>,
    pub card_id: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Webhook {
    pub id: String,
    pub name: String,
    pub url: String,
    /// Wird nie an die UI ausgeliefert (nur `has_secret`).
    #[serde(skip_serializing, default)]
    pub secret: Option<String>,
    #[cfg_attr(feature = "server", sqlx(json))]
    pub events: Vec<String>,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub last_status: Option<i64>,
    pub last_error: Option<String>,
    pub last_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewWebhook {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub secret: Option<String>,
    /// Leer = alle Ereignisse.
    #[serde(default)]
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchResults {
    pub boards: Vec<Board>,
    pub cards: Vec<CardRef>,
    pub todos: Vec<Todo>,
    pub events: Vec<Event>,
}
