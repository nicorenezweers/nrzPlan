//! Aktivitätslog + ausgehende Webhooks.
//!
//! Jede Änderung (Karte, Todo, Termin) ruft `emit` auf: Eintrag ins Aktivitätslog und – falls
//! konfiguriert – POST an alle passenden Webhooks (asynchron, blockiert die Anfrage nicht).

use std::sync::OnceLock;
use std::time::Duration;

use chrono::Local;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::model::Webhook;

tokio::task_local! {
    /// Wer die Änderung auslöst ("nrzPlan" für die UI, "API · <Key-Name>" für die REST-API).
    pub static ACTOR: String;
}

pub fn actor() -> String {
    ACTOR.try_with(|a| a.clone()).unwrap_or_else(|_| "nrzPlan".into())
}

fn now_ts() -> String {
    Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Alle Ereignistypen (für die Webhook-Auswahl in den Einstellungen).
pub const EVENT_KINDS: [&str; 11] = [
    "card.created",
    "card.updated",
    "card.moved",
    "card.deleted",
    "todo.created",
    "todo.updated",
    "todo.completed",
    "todo.deleted",
    "event.created",
    "event.updated",
    "event.deleted",
];

pub struct Emit<'a> {
    pub kind: &'a str,
    pub message: String,
    pub board_id: Option<&'a str>,
    pub card_id: Option<&'a str>,
    pub data: Value,
}

/// Protokolliert eine Änderung und verschickt Webhooks. Fehler werden nur geloggt.
pub async fn emit(pool: &SqlitePool, e: Emit<'_>) {
    let actor = actor();
    let at = now_ts();
    let res = sqlx::query(
        "INSERT INTO activity (id, kind, message, actor, board_id, card_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(e.kind)
    .bind(&e.message)
    .bind(&actor)
    .bind(e.board_id)
    .bind(e.card_id)
    .bind(&at)
    .execute(pool)
    .await;
    if let Err(err) = res {
        tracing::warn!("Aktivität konnte nicht gespeichert werden: {err}");
    }

    let hooks = match sqlx::query_as::<_, Webhook>(
        "SELECT id, name, url, secret, events, active, created_at, last_status, last_error, last_at
         FROM webhooks WHERE active = 1",
    )
    .fetch_all(pool)
    .await
    {
        Ok(h) => h,
        Err(err) => {
            tracing::warn!("Webhooks konnten nicht geladen werden: {err}");
            return;
        }
    };
    let payload = json!({
        "event": e.kind,
        "message": e.message,
        "actor": actor,
        "at": at,
        "board_id": e.board_id,
        "data": e.data,
    });
    for hook in hooks.into_iter().filter(|h| matches(h, e.kind)) {
        let pool = pool.clone();
        let payload = payload.clone();
        tokio::spawn(async move {
            deliver(&pool, &hook, e_kind(&payload), &payload).await;
        });
    }
}

fn e_kind(payload: &Value) -> String {
    payload["event"].as_str().unwrap_or_default().to_string()
}

/// Leere Liste = alle Ereignisse; `card.*` = alle Karten-Ereignisse.
fn matches(hook: &Webhook, kind: &str) -> bool {
    hook.events.is_empty()
        || hook.events.iter().any(|ev| {
            ev == kind || ev == "*" || ev.strip_suffix(".*").is_some_and(|p| kind.starts_with(&format!("{p}.")))
        })
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("nrzPlan-Webhook/1")
            .build()
            .unwrap_or_default()
    })
}

fn sign(secret: &str, body: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else { return String::new() };
    mac.update(body);
    mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

/// Schickt einen Webhook und merkt sich das Ergebnis. Rückgabe: (HTTP-Status, Fehler).
pub async fn deliver(pool: &SqlitePool, hook: &Webhook, kind: String, payload: &Value) -> (Option<u16>, Option<String>) {
    let body = serde_json::to_vec(payload).unwrap_or_default();
    let mut req = client()
        .post(&hook.url)
        .header("Content-Type", "application/json")
        .header("X-NrzPlan-Event", &kind)
        .header("X-NrzPlan-Delivery", Uuid::new_v4().to_string());
    if let Some(secret) = hook.secret.as_deref().filter(|s| !s.is_empty()) {
        req = req.header("X-NrzPlan-Signature", format!("sha256={}", sign(secret, &body)));
    }
    let (status, error) = match req.body(body).send().await {
        Ok(r) if r.status().is_success() => (Some(r.status().as_u16()), None),
        Ok(r) => (Some(r.status().as_u16()), Some(format!("HTTP {}", r.status()))),
        Err(e) => (None, Some(e.to_string())),
    };
    let _ = sqlx::query("UPDATE webhooks SET last_status = ?, last_error = ?, last_at = ? WHERE id = ?")
        .bind(status.map(|s| s as i64))
        .bind(&error)
        .bind(now_ts())
        .bind(&hook.id)
        .execute(pool)
        .await;
    (status, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hook(events: &[&str]) -> Webhook {
        Webhook {
            id: String::new(),
            name: String::new(),
            url: String::new(),
            secret: None,
            events: events.iter().map(|s| s.to_string()).collect(),
            active: true,
            created_at: chrono::NaiveDateTime::default(),
            last_status: None,
            last_error: None,
            last_at: None,
        }
    }

    #[test]
    fn event_matching() {
        assert!(matches(&hook(&[]), "card.moved"));
        assert!(matches(&hook(&["card.*"]), "card.moved"));
        assert!(!matches(&hook(&["card.*"]), "todo.created"));
        assert!(matches(&hook(&["todo.completed"]), "todo.completed"));
        assert!(!matches(&hook(&["todo.completed"]), "todo.created"));
    }

    #[test]
    fn signature() {
        // printf '{}' | openssl dgst -sha256 -hmac geheim
        assert_eq!(sign("geheim", b"{}"), "1d88585cae3e449a08c782eea7789eff15b9d3776aa7f0253181df1efe8b191d");
    }
}
