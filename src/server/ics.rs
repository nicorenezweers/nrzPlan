//! Kalender-Abo (iCalendar, RFC 5545) für Apple/Google/Outlook-Kalender.

use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use sqlx::SqlitePool;

use super::AppError;
use super::db;

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace("\r\n", "\\n").replace('\n', "\\n")
}

/// Zeilen über 75 Bytes falten (Fortsetzung mit Leerzeichen).
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut len = 0;
    for ch in line.chars() {
        let w = ch.len_utf8();
        if len + w > 75 {
            out.push_str("\r\n ");
            len = 1;
        }
        out.push(ch);
        len += w;
    }
    out.push_str("\r\n");
    out
}

fn dt(d: NaiveDateTime) -> String {
    d.format("%Y%m%dT%H%M%S").to_string()
}

fn date(d: NaiveDate) -> String {
    d.format("%Y%m%d").to_string()
}

struct Builder {
    out: String,
    stamp: String,
}

impl Builder {
    fn line(&mut self, l: &str) {
        self.out.push_str(&fold(l));
    }

    fn all_day(&mut self, uid: &str, title: &str, start: NaiveDate, end_incl: NaiveDate, desc: Option<&str>) {
        self.line("BEGIN:VEVENT");
        self.line(&format!("UID:{uid}@nrzplan"));
        self.line(&format!("DTSTAMP:{}", self.stamp));
        self.line(&format!("DTSTART;VALUE=DATE:{}", date(start)));
        self.line(&format!("DTEND;VALUE=DATE:{}", date(end_incl + Duration::days(1))));
        self.line(&format!("SUMMARY:{}", esc(title)));
        if let Some(d) = desc {
            self.line(&format!("DESCRIPTION:{}", esc(d)));
        }
        self.line("TRANSP:TRANSPARENT");
        self.line("END:VEVENT");
    }
}

/// Termine (−60 bis +365 Tage), fällige Todos und Board-Karten als VEVENTs.
pub async fn build(pool: &SqlitePool) -> Result<String, AppError> {
    let today = Local::now().date_naive();
    let data = db::calendar(pool, today - Duration::days(60), today + Duration::days(365)).await?;
    let mut b = Builder { out: String::new(), stamp: dt(Local::now().naive_local()) };
    b.line("BEGIN:VCALENDAR");
    b.line("VERSION:2.0");
    b.line("PRODID:-//nrzPlan//DE");
    b.line("CALSCALE:GREGORIAN");
    b.line("METHOD:PUBLISH");
    b.line("X-WR-CALNAME:nrzPlan");
    b.line("X-PUBLISHED-TTL:PT15M");
    b.line("REFRESH-INTERVAL;VALUE=DURATION:PT15M");

    for e in &data.events {
        if e.all_day {
            b.all_day(&e.id, &e.title, e.start_at.date(), e.end_at.date(), e.description.as_deref());
            continue;
        }
        b.line("BEGIN:VEVENT");
        b.line(&format!("UID:{}@nrzplan", e.id));
        b.line(&format!("DTSTAMP:{}", b.stamp));
        // "Floating" Ortszeit – wird in der Zeitzone des Abonnenten angezeigt.
        b.line(&format!("DTSTART:{}", dt(e.start_at)));
        b.line(&format!("DTEND:{}", dt(e.end_at)));
        b.line(&format!("SUMMARY:{}", esc(&e.title)));
        if let Some(l) = &e.location {
            b.line(&format!("LOCATION:{}", esc(l)));
        }
        if let Some(d) = &e.description {
            b.line(&format!("DESCRIPTION:{}", esc(d)));
        }
        b.line("END:VEVENT");
    }
    for t in &data.todos {
        if let Some(d) = t.due_date {
            let title = format!("{} {}", if t.done { "✓" } else { "☐" }, t.title);
            b.all_day(&format!("todo-{}", t.id), &title, d, d, t.notes.as_deref());
        }
    }
    for c in &data.cards {
        if let Some(d) = c.card.due_date {
            let title = format!("▢ {} ({})", c.card.title, c.board_name);
            let desc = format!("{} · {}", c.board_name, c.column_name);
            b.all_day(&format!("card-{}", c.card.id), &title, d, d, Some(&desc));
        }
    }
    b.line("END:VCALENDAR");
    Ok(b.out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding_and_escaping() {
        assert_eq!(esc("a,b;c\nd"), "a\\,b\\;c\\nd");
        let long = "SUMMARY:".to_string() + &"ä".repeat(60);
        let folded = fold(&long);
        assert!(folded.split("\r\n").all(|l| l.len() <= 75));
        assert_eq!(folded.replace("\r\n ", "").trim_end(), long);
    }
}
