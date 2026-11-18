//! Server-Funktionen für die UI (intern; extern bitte die REST-API unter `/api/v1` nutzen).

use chrono::NaiveDate;
use dioxus::prelude::*;

use crate::model::*;
use crate::model::Event;

#[cfg(feature = "server")]
use crate::server::{db, pool};

#[cfg(feature = "server")]
fn e(err: crate::server::AppError) -> ServerFnError {
    ServerFnError::new(err.to_string())
}

// --- Boards ---------------------------------------------------------------

#[server]
pub async fn get_boards() -> Result<Vec<BoardSummary>, ServerFnError> {
    db::list_boards(pool()).await.map_err(e)
}

#[server]
pub async fn create_board(name: String) -> Result<Board, ServerFnError> {
    db::create_board(pool(), NewBoard { name, ..Default::default() }).await.map_err(e)
}

#[server]
pub async fn update_board(id: String, name: String, description: Option<String>) -> Result<Board, ServerFnError> {
    db::update_board(pool(), &id, name, description).await.map_err(e)
}

#[server]
pub async fn delete_board(id: String) -> Result<(), ServerFnError> {
    db::delete_board(pool(), &id).await.map_err(e)
}

#[server]
pub async fn get_board(id: String) -> Result<BoardFull, ServerFnError> {
    db::get_board_full(pool(), &id).await.map_err(e)
}

#[server]
pub async fn create_column(board_id: String, name: String) -> Result<Column, ServerFnError> {
    db::create_column(pool(), &board_id, name).await.map_err(e)
}

#[server]
pub async fn update_column(id: String, name: Option<String>, is_done: Option<bool>) -> Result<(), ServerFnError> {
    db::update_column(pool(), &id, name, is_done).await.map_err(e)
}

#[server]
pub async fn move_column(id: String, index: i64) -> Result<(), ServerFnError> {
    db::move_column(pool(), &id, index).await.map_err(e)
}

#[server]
pub async fn delete_column(id: String) -> Result<(), ServerFnError> {
    db::delete_column(pool(), &id).await.map_err(e)
}

#[server]
pub async fn set_label_color(id: String, color: String) -> Result<(), ServerFnError> {
    db::set_label_color(pool(), &id, &color).await.map_err(e)
}

// --- Karten ---------------------------------------------------------------

#[server]
pub async fn create_card(board_id: String, input: NewCard) -> Result<Card, ServerFnError> {
    db::create_card(pool(), &board_id, input).await.map(|(c, _)| c).map_err(e)
}

#[server]
pub async fn get_card(id: String) -> Result<CardDetail, ServerFnError> {
    db::get_card_detail(pool(), &id).await.map_err(e)
}

#[server]
pub async fn update_card(id: String, patch: CardPatch) -> Result<Card, ServerFnError> {
    db::update_card(pool(), &id, patch).await.map_err(e)
}

#[server]
pub async fn move_card(id: String, column_id: String, index: i64) -> Result<(), ServerFnError> {
    db::move_card(pool(), &id, &column_id, index).await.map_err(e)
}

#[server]
pub async fn delete_card(id: String) -> Result<(), ServerFnError> {
    db::delete_card(pool(), &id).await.map_err(e)
}

#[server]
pub async fn add_checklist_item(card_id: String, text: String) -> Result<ChecklistItem, ServerFnError> {
    db::add_checklist_item(pool(), &card_id, text).await.map_err(e)
}

#[server]
pub async fn set_checklist_item(id: String, done: bool) -> Result<(), ServerFnError> {
    db::set_checklist_item(pool(), &id, done).await.map_err(e)
}

#[server]
pub async fn delete_checklist_item(id: String) -> Result<(), ServerFnError> {
    db::delete_checklist_item(pool(), &id).await.map_err(e)
}

// --- Todos ----------------------------------------------------------------

#[server]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    db::list_todos_ui(pool()).await.map_err(e)
}

#[server]
pub async fn create_todo(input: NewTodo) -> Result<Todo, ServerFnError> {
    db::create_todo(pool(), input).await.map(|(t, _)| t).map_err(e)
}

#[server]
pub async fn update_todo(id: String, patch: TodoPatch) -> Result<Todo, ServerFnError> {
    db::update_todo(pool(), &id, patch).await.map_err(e)
}

#[server]
pub async fn delete_todo(id: String) -> Result<(), ServerFnError> {
    db::delete_todo(pool(), &id).await.map_err(e)
}

// --- Kalender -------------------------------------------------------------

#[server]
pub async fn get_calendar(from: NaiveDate, to: NaiveDate) -> Result<CalendarData, ServerFnError> {
    db::calendar(pool(), from, to).await.map_err(e)
}

#[server]
pub async fn create_event(input: NewEvent) -> Result<Event, ServerFnError> {
    db::create_event(pool(), input).await.map(|(ev, _)| ev).map_err(e)
}

#[server]
pub async fn update_event(id: String, patch: EventPatch) -> Result<Event, ServerFnError> {
    db::update_event(pool(), &id, patch).await.map_err(e)
}

#[server]
pub async fn delete_event(id: String) -> Result<(), ServerFnError> {
    db::delete_event(pool(), &id).await.map_err(e)
}

#[server]
pub async fn get_today(date: NaiveDate) -> Result<TodayData, ServerFnError> {
    db::today(pool(), date).await.map_err(e)
}

// --- API-Keys -------------------------------------------------------------

#[server]
pub async fn get_api_keys() -> Result<Vec<ApiKey>, ServerFnError> {
    db::list_api_keys(pool()).await.map_err(e)
}

#[server]
pub async fn create_api_key(name: String) -> Result<CreatedApiKey, ServerFnError> {
    db::create_api_key(pool(), name).await.map_err(e)
}

#[server]
pub async fn delete_api_key(id: String) -> Result<(), ServerFnError> {
    db::delete_api_key(pool(), &id).await.map_err(e)
}

// --- Aktivität, Webhooks, Suche, Kalender-Abo ------------------------------

#[server]
pub async fn get_activity(board_id: Option<String>, limit: i64) -> Result<Vec<Activity>, ServerFnError> {
    db::list_activity(pool(), board_id.as_deref(), limit).await.map_err(e)
}

#[server]
pub async fn search(q: String) -> Result<SearchResults, ServerFnError> {
    db::search(pool(), &q).await.map_err(e)
}

#[server]
pub async fn get_webhooks() -> Result<Vec<Webhook>, ServerFnError> {
    db::list_webhooks(pool()).await.map_err(e)
}

#[server]
pub async fn get_webhook_events() -> Result<Vec<String>, ServerFnError> {
    Ok(crate::server::events::EVENT_KINDS.iter().map(|s| s.to_string()).collect())
}

#[server]
pub async fn create_webhook(input: NewWebhook) -> Result<Webhook, ServerFnError> {
    db::create_webhook(pool(), input).await.map_err(e)
}

#[server]
pub async fn set_webhook_active(id: String, active: bool) -> Result<(), ServerFnError> {
    db::set_webhook_active(pool(), &id, active).await.map_err(e)
}

#[server]
pub async fn delete_webhook(id: String) -> Result<(), ServerFnError> {
    db::delete_webhook(pool(), &id).await.map_err(e)
}

/// Schickt ein `ping`-Ereignis; Rückgabe: Statusmeldung.
#[server]
pub async fn test_webhook(id: String) -> Result<String, ServerFnError> {
    let hook = db::get_webhook(pool(), &id).await.map_err(e)?;
    let payload = serde_json::json!({
        "event": "ping",
        "message": "Test von nrzPlan",
        "actor": "nrzPlan",
        "at": chrono::Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "data": {},
    });
    let (status, err) = crate::server::events::deliver(pool(), &hook, "ping".into(), &payload).await;
    Ok(match (status, err) {
        (Some(s), None) => format!("OK – HTTP {s}"),
        (_, Some(err)) => format!("Fehler: {err}"),
        _ => "Unbekannt".into(),
    })
}

#[server]
pub async fn get_ics_token(rotate: bool) -> Result<String, ServerFnError> {
    db::ics_token(pool(), rotate).await.map_err(e)
}
