//! Öffentliche REST-API unter `/api/v1` – für externe Tools (Skripte, CI, Shortcuts, n8n, …).
//!
//! Auth: `Authorization: Bearer <key>` oder `X-API-Key: <key>`.
//! Keys werden in den Einstellungen erzeugt; zusätzlich gilt `NRZPLAN_API_KEY` aus der Umgebung.

use chrono::{Local, NaiveDate, NaiveDateTime};
use dioxus::server::axum::extract::{FromRequestParts, Path, Query};
use dioxus::server::axum::extract::Request;
use dioxus::server::axum::http::request::Parts;
use dioxus::server::axum::http::{HeaderMap, StatusCode, header};
use dioxus::server::axum::middleware::{self, Next};
use dioxus::server::axum::response::{IntoResponse, Response};
use dioxus::server::axum::routing::{get, patch, post};
use dioxus::server::axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::db::{self, CardFilter, TodoFilter};
use super::events::ACTOR;
use super::{AppError, ics, pool};
use crate::model::*;

type ApiResult<T> = Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}

/// Markiert eine Anfrage als authentifiziert (gesetzt von `auth_middleware`).
#[derive(Clone)]
struct Authed;

/// Extraktor: nur erfolgreich, wenn die Auth-Middleware die Anfrage geprüft hat.
pub struct ApiAuth;

impl<S: Send + Sync> FromRequestParts<S> for ApiAuth {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Authed>().map(|_| ApiAuth).ok_or(AppError::Unauthorized)
    }
}

/// Liefert den Namen des API-Keys oder `Unauthorized`.
async fn authenticate(headers: &HeaderMap) -> Result<String, AppError> {
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).map(str::trim);
    let key = header("authorization")
        .and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")))
        .or_else(|| header("x-api-key"))
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .ok_or(AppError::Unauthorized)?;
    if let Ok(env_key) = std::env::var("NRZPLAN_API_KEY")
        && !env_key.is_empty()
        && env_key == key
    {
        return Ok("NRZPLAN_API_KEY".into());
    }
    db::verify_api_key(pool(), key).await?.ok_or(AppError::Unauthorized)
}

/// Prüft den API-Key und führt den Handler mit Akteur "API · <Key-Name>" aus (für Aktivitätslog/Webhooks).
async fn auth_middleware(mut req: Request, next: Next) -> Response {
    match authenticate(req.headers()).await {
        Ok(name) => {
            req.extensions_mut().insert(Authed);
            ACTOR.scope(format!("API · {name}"), next.run(req)).await
        }
        Err(e) => e.into_response(),
    }
}

fn created<T: serde::Serialize>(value: T, is_new: bool) -> Response {
    let status = if is_new { StatusCode::CREATED } else { StatusCode::OK };
    (status, Json(value)).into_response()
}

pub fn router() -> Router {
    let public = Router::new()
        .route("/", get(index))
        .route("/health", get(|| async { Json(json!({ "ok": true })) }))
        .route("/calendar.ics", get(calendar_ics));
    let protected = Router::new()
        .route("/boards", get(list_boards).post(create_board))
        .route("/boards/{board}", get(get_board).patch(update_board).delete(delete_board))
        .route("/boards/{board}/columns", get(list_columns).post(create_column))
        .route("/boards/{board}/cards", get(list_cards).post(create_card))
        .route(
            "/boards/{board}/cards/external/{source}/{external_id}",
            get(get_card_external).patch(patch_card_external).delete(delete_card_external),
        )
        .route("/cards/{id}", get(get_card).patch(patch_card).delete(delete_card))
        .route("/cards/{id}/move", post(move_card))
        .route("/cards/{id}/checklist", post(add_checklist))
        .route("/todos", get(list_todos).post(create_todo))
        .route("/todos/{id}", get(get_todo).patch(patch_todo).delete(delete_todo))
        .route("/todos/{id}/done", post(done_todo))
        .route("/events", get(list_events).post(create_event))
        .route("/events/{id}", get(get_event).patch(patch_event).delete(delete_event))
        .route("/agenda", get(agenda))
        .route("/columns/{id}", patch(update_column))
        .route("/search", get(search))
        .route("/activity", get(activity))
        .route("/boards/{board}/activity", get(board_activity))
        .route_layer(middleware::from_fn(auth_middleware));
    public.merge(protected)
}

async fn index() -> Json<Value> {
    Json(json!({
        "name": "nrzPlan API",
        "version": "v1",
        "auth": "Authorization: Bearer <key>  oder  X-API-Key: <key>",
        "endpoints": [
            "GET    /api/v1/boards",
            "POST   /api/v1/boards                      {name, description?, columns?[]}",
            "GET    /api/v1/boards/{board}              (board = ID oder Name)",
            "PATCH  /api/v1/boards/{board}              {name?, description?}",
            "DELETE /api/v1/boards/{board}",
            "GET    /api/v1/boards/{board}/columns",
            "POST   /api/v1/boards/{board}/columns      {name}",
            "PATCH  /api/v1/columns/{id}                {name?, is_done?, position?}",
            "GET    /api/v1/boards/{board}/cards        ?column=&label=&source=&external_id=&q=",
            "POST   /api/v1/boards/{board}/cards        {title, column|status?, description?, labels?[], priority?, due_date?, url?, source?, external_id?, checklist?[]}",
            "GET    /api/v1/boards/{board}/cards/external/{source}/{external_id}",
            "PATCH  /api/v1/boards/{board}/cards/external/{source}/{external_id}",
            "GET    /api/v1/cards/{id}",
            "PATCH  /api/v1/cards/{id}                  {title?, column|status?, description?, labels?, priority?, due_date?, url?, position?}",
            "POST   /api/v1/cards/{id}/move             {column, position?}",
            "POST   /api/v1/cards/{id}/checklist        {text}",
            "DELETE /api/v1/cards/{id}",
            "GET    /api/v1/todos                       ?done=&list=&due_before=&source=&external_id=",
            "POST   /api/v1/todos                       {title, notes?, list?, due_date?, priority?, recurrence?, source?, external_id?}",
            "PATCH  /api/v1/todos/{id}                  {title?, notes?, list?, due_date?, priority?, recurrence?, done?}",
            "POST   /api/v1/todos/{id}/done",
            "DELETE /api/v1/todos/{id}",
            "GET    /api/v1/events                      ?from=YYYY-MM-DD&to=YYYY-MM-DD",
            "POST   /api/v1/events                      {title, start_at, end_at?, all_day?, location?, description?, color?, source?, external_id?}",
            "PATCH  /api/v1/events/{id}",
            "DELETE /api/v1/events/{id}",
            "GET    /api/v1/agenda                      ?date=YYYY-MM-DD",
            "GET    /api/v1/search                      ?q=",
            "GET    /api/v1/activity                    ?limit=",
            "GET    /api/v1/boards/{board}/activity     ?limit=",
            "GET    /api/v1/calendar.ics?token=…        (Kalender-Abo, Token in den Einstellungen)"
        ],
        "todo_recurrence": ["daily", "weekdays", "weekly", "monthly", "yearly"],
        "webhook_events": super::events::EVENT_KINDS
    }))
}

// ---------------------------------------------------------------------------
// Boards & Spalten
// ---------------------------------------------------------------------------

async fn list_boards(_: ApiAuth) -> ApiResult<Json<Vec<BoardSummary>>> {
    Ok(Json(db::list_boards(pool()).await?))
}

async fn create_board(_: ApiAuth, Json(input): Json<NewBoard>) -> ApiResult<Response> {
    let board = db::create_board(pool(), input).await?;
    Ok(created(db::get_board_full(pool(), &board.id).await?, true))
}

async fn get_board(_: ApiAuth, Path(board): Path<String>) -> ApiResult<Json<BoardFull>> {
    Ok(Json(db::get_board_full(pool(), &board).await?))
}

#[derive(Deserialize)]
struct BoardPatch {
    name: Option<String>,
    description: Option<String>,
}

async fn update_board(_: ApiAuth, Path(board): Path<String>, Json(p): Json<BoardPatch>) -> ApiResult<Json<Board>> {
    let b = db::resolve_board(pool(), &board).await?;
    let name = p.name.unwrap_or(b.name);
    let desc = p.description.or(b.description);
    Ok(Json(db::update_board(pool(), &b.id, name, desc).await?))
}

async fn delete_board(_: ApiAuth, Path(board): Path<String>) -> ApiResult<StatusCode> {
    let b = db::resolve_board(pool(), &board).await?;
    db::delete_board(pool(), &b.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_columns(_: ApiAuth, Path(board): Path<String>) -> ApiResult<Json<Vec<Column>>> {
    Ok(Json(db::get_board_full(pool(), &board).await?.columns))
}

#[derive(Deserialize)]
struct NameBody {
    name: String,
}

async fn create_column(_: ApiAuth, Path(board): Path<String>, Json(b): Json<NameBody>) -> ApiResult<Response> {
    let board = db::resolve_board(pool(), &board).await?;
    Ok(created(db::create_column(pool(), &board.id, b.name).await?, true))
}

#[derive(Deserialize)]
struct ColumnPatch {
    name: Option<String>,
    is_done: Option<bool>,
    position: Option<i64>,
}

async fn update_column(_: ApiAuth, Path(id): Path<String>, Json(p): Json<ColumnPatch>) -> ApiResult<StatusCode> {
    db::update_column(pool(), &id, p.name, p.is_done).await?;
    if let Some(pos) = p.position {
        db::move_column(pool(), &id, pos).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Karten
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CardQuery {
    #[serde(alias = "status")]
    column: Option<String>,
    label: Option<String>,
    source: Option<String>,
    external_id: Option<String>,
    q: Option<String>,
}

async fn list_cards(_: ApiAuth, Path(board): Path<String>, Query(q): Query<CardQuery>) -> ApiResult<Json<Vec<Card>>> {
    let f = CardFilter { column: q.column, label: q.label, source: q.source, external_id: q.external_id, q: q.q };
    Ok(Json(db::list_cards(pool(), &board, f).await?))
}

async fn create_card(_: ApiAuth, Path(board): Path<String>, Json(input): Json<NewCard>) -> ApiResult<Response> {
    let (card, is_new) = db::create_card(pool(), &board, input).await?;
    Ok(created(db::get_card_detail(pool(), &card.id).await?, is_new))
}

async fn get_card(_: ApiAuth, Path(id): Path<String>) -> ApiResult<Json<CardDetail>> {
    Ok(Json(db::get_card_detail(pool(), &id).await?))
}

async fn patch_card(_: ApiAuth, Path(id): Path<String>, Json(p): Json<CardPatch>) -> ApiResult<Json<CardDetail>> {
    db::update_card(pool(), &id, p).await?;
    Ok(Json(db::get_card_detail(pool(), &id).await?))
}

async fn delete_card(_: ApiAuth, Path(id): Path<String>) -> ApiResult<StatusCode> {
    db::delete_card(pool(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn external_card(board: &str, source: &str, external_id: &str) -> ApiResult<Card> {
    let b = db::resolve_board(pool(), board).await?;
    db::find_card_external(pool(), &b.id, source, external_id).await
}

async fn get_card_external(
    _: ApiAuth,
    Path((board, source, external_id)): Path<(String, String, String)>,
) -> ApiResult<Json<CardDetail>> {
    let card = external_card(&board, &source, &external_id).await?;
    Ok(Json(db::get_card_detail(pool(), &card.id).await?))
}

async fn patch_card_external(
    _: ApiAuth,
    Path((board, source, external_id)): Path<(String, String, String)>,
    Json(p): Json<CardPatch>,
) -> ApiResult<Json<CardDetail>> {
    let card = external_card(&board, &source, &external_id).await?;
    db::update_card(pool(), &card.id, p).await?;
    Ok(Json(db::get_card_detail(pool(), &card.id).await?))
}

async fn delete_card_external(
    _: ApiAuth,
    Path((board, source, external_id)): Path<(String, String, String)>,
) -> ApiResult<StatusCode> {
    let card = external_card(&board, &source, &external_id).await?;
    db::delete_card(pool(), &card.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct MoveBody {
    #[serde(alias = "status")]
    column: String,
    position: Option<i64>,
}

async fn move_card(_: ApiAuth, Path(id): Path<String>, Json(b): Json<MoveBody>) -> ApiResult<Json<CardDetail>> {
    let patch = CardPatch { column: Some(b.column), position: b.position, ..Default::default() };
    db::update_card(pool(), &id, patch).await?;
    Ok(Json(db::get_card_detail(pool(), &id).await?))
}

#[derive(Deserialize)]
struct TextBody {
    text: String,
}

async fn add_checklist(_: ApiAuth, Path(id): Path<String>, Json(b): Json<TextBody>) -> ApiResult<Response> {
    db::get_card(pool(), &id).await?;
    Ok(created(db::add_checklist_item(pool(), &id, b.text).await?, true))
}

// ---------------------------------------------------------------------------
// Todos
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct TodoQuery {
    done: Option<bool>,
    list: Option<String>,
    due_before: Option<NaiveDate>,
    source: Option<String>,
    external_id: Option<String>,
}

async fn list_todos(_: ApiAuth, Query(q): Query<TodoQuery>) -> ApiResult<Json<Vec<Todo>>> {
    let f = TodoFilter {
        done: q.done,
        list: q.list,
        due_before: q.due_before,
        source: q.source,
        external_id: q.external_id,
    };
    Ok(Json(db::list_todos(pool(), f).await?))
}

async fn create_todo(_: ApiAuth, Json(input): Json<NewTodo>) -> ApiResult<Response> {
    let (todo, is_new) = db::create_todo(pool(), input).await?;
    Ok(created(todo, is_new))
}

async fn get_todo(_: ApiAuth, Path(id): Path<String>) -> ApiResult<Json<Todo>> {
    Ok(Json(db::get_todo(pool(), &id).await?))
}

async fn patch_todo(_: ApiAuth, Path(id): Path<String>, Json(p): Json<TodoPatch>) -> ApiResult<Json<Todo>> {
    Ok(Json(db::update_todo(pool(), &id, p).await?))
}

async fn done_todo(_: ApiAuth, Path(id): Path<String>) -> ApiResult<Json<Todo>> {
    Ok(Json(db::update_todo(pool(), &id, TodoPatch { done: Some(true), ..Default::default() }).await?))
}

async fn delete_todo(_: ApiAuth, Path(id): Path<String>) -> ApiResult<StatusCode> {
    db::delete_todo(pool(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Termine & Agenda
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RangeQuery {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

fn day_start(d: NaiveDate) -> NaiveDateTime {
    d.and_hms_opt(0, 0, 0).unwrap_or_default()
}

async fn list_events(_: ApiAuth, Query(q): Query<RangeQuery>) -> ApiResult<Json<Vec<Event>>> {
    let today = Local::now().date_naive();
    let from = q.from.unwrap_or(today);
    let to = q.to.unwrap_or(from + chrono::Duration::days(30));
    Ok(Json(db::list_events(pool(), day_start(from), day_start(to + chrono::Duration::days(1))).await?))
}

async fn create_event(_: ApiAuth, Json(input): Json<NewEvent>) -> ApiResult<Response> {
    let (ev, is_new) = db::create_event(pool(), input).await?;
    Ok(created(ev, is_new))
}

async fn get_event(_: ApiAuth, Path(id): Path<String>) -> ApiResult<Json<Event>> {
    Ok(Json(db::get_event(pool(), &id).await?))
}

async fn patch_event(_: ApiAuth, Path(id): Path<String>, Json(p): Json<EventPatch>) -> ApiResult<Json<Event>> {
    Ok(Json(db::update_event(pool(), &id, p).await?))
}

async fn delete_event(_: ApiAuth, Path(id): Path<String>) -> ApiResult<StatusCode> {
    db::delete_event(pool(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct DateQuery {
    date: Option<NaiveDate>,
}

async fn agenda(_: ApiAuth, Query(q): Query<DateQuery>) -> ApiResult<Json<TodayData>> {
    Ok(Json(db::today(pool(), q.date.unwrap_or_else(|| Local::now().date_naive())).await?))
}

// ---------------------------------------------------------------------------
// Suche, Aktivität, Kalender-Abo
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SearchQuery {
    #[serde(default)]
    q: String,
}

async fn search(_: ApiAuth, Query(q): Query<SearchQuery>) -> ApiResult<Json<SearchResults>> {
    Ok(Json(db::search(pool(), &q.q).await?))
}

#[derive(Deserialize)]
struct LimitQuery {
    limit: Option<i64>,
}

async fn activity(_: ApiAuth, Query(q): Query<LimitQuery>) -> ApiResult<Json<Vec<Activity>>> {
    Ok(Json(db::list_activity(pool(), None, q.limit.unwrap_or(50)).await?))
}

async fn board_activity(
    _: ApiAuth,
    Path(board): Path<String>,
    Query(q): Query<LimitQuery>,
) -> ApiResult<Json<Vec<Activity>>> {
    let b = db::resolve_board(pool(), &board).await?;
    Ok(Json(db::list_activity(pool(), Some(&b.id), q.limit.unwrap_or(50)).await?))
}

#[derive(Deserialize)]
struct TokenQuery {
    #[serde(default)]
    token: String,
}

/// Öffentlich erreichbar, aber nur mit dem geheimen Abo-Token aus den Einstellungen.
async fn calendar_ics(Query(q): Query<TokenQuery>) -> ApiResult<Response> {
    let expected = db::get_setting(pool(), "ics_token").await?;
    if q.token.is_empty() || expected.as_deref() != Some(q.token.as_str()) {
        return Err(AppError::Unauthorized);
    }
    let body = ics::build(pool()).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "inline; filename=\"nrzplan.ics\""),
        ],
        body,
    )
        .into_response())
}
