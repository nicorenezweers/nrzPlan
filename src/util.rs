//! Hilfsfunktionen: Datum (deutsch), Farben, Schnell-Eingabe.

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, Weekday};

pub const WEEKDAYS_SHORT: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];
pub const WEEKDAYS: [&str; 7] = ["Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag", "Sonntag"];
pub const MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// "Freitag, 13. November"
pub fn fmt_long(d: NaiveDate) -> String {
    format!("{}, {}. {}", WEEKDAYS[d.weekday().num_days_from_monday() as usize], d.day(), MONTHS[d.month0() as usize])
}

/// "13. Nov."
pub fn fmt_short(d: NaiveDate) -> String {
    let m = MONTHS[d.month0() as usize];
    let m: String = if m.chars().count() > 4 { format!("{}.", m.chars().take(3).collect::<String>()) } else { m.into() };
    format!("{}. {}", d.day(), m)
}

/// Relativ: "Heute", "Morgen", "Gestern", Wochentag (innerhalb 6 Tagen) oder kurzes Datum.
pub fn fmt_relative(d: NaiveDate) -> String {
    let diff = (d - today()).num_days();
    match diff {
        0 => "Heute".into(),
        1 => "Morgen".into(),
        -1 => "Gestern".into(),
        2..=6 => WEEKDAYS[d.weekday().num_days_from_monday() as usize].into(),
        _ => fmt_short(d),
    }
}

pub fn fmt_time(dt: NaiveDateTime) -> String {
    dt.format("%H:%M").to_string()
}

/// "gerade eben", "vor 5 Min.", "vor 3 Std.", "gestern 14:02", "12.11. 09:30"
pub fn fmt_ago(dt: NaiveDateTime) -> String {
    let now = Local::now().naive_local();
    let mins = (now - dt).num_minutes();
    match mins {
        i64::MIN..=0 => "gerade eben".into(),
        1..=59 => format!("vor {mins} Min."),
        60..=719 => format!("vor {} Std.", mins / 60),
        _ if dt.date() == now.date() => format!("heute {}", fmt_time(dt)),
        _ if dt.date() == now.date() - Duration::days(1) => format!("gestern {}", fmt_time(dt)),
        _ => dt.format("%d.%m. %H:%M").to_string(),
    }
}

pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

pub fn date_input(d: Option<NaiveDate>) -> String {
    d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
}

pub fn greeting() -> &'static str {
    use chrono::Timelike;
    match Local::now().hour() {
        5..=10 => "Guten Morgen",
        11..=17 => "Guten Tag",
        18..=22 => "Guten Abend",
        _ => "Gute Nacht",
    }
}

/// Erster Tag (Montag) der Woche von `d`.
pub fn week_start(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

/// 6×7-Raster für die Monatsansicht (beginnt am Montag).
pub fn month_grid(year: i32, month: u32) -> Vec<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_else(today);
    let start = week_start(first);
    (0..42).map(|i| start + Duration::days(i)).collect()
}

pub fn add_months(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let idx = year * 12 + month as i32 - 1 + delta;
    (idx.div_euclid(12), (idx.rem_euclid(12) + 1) as u32)
}

// ---------------------------------------------------------------------------
// Farben (vollständige Klassennamen, damit Tailwind sie findet)
// ---------------------------------------------------------------------------

pub fn dot_class(color: &str) -> &'static str {
    match color {
        "blue" => "bg-blue-500",
        "green" => "bg-emerald-500",
        "orange" => "bg-orange-500",
        "red" => "bg-red-500",
        "purple" => "bg-violet-500",
        "yellow" => "bg-amber-400",
        "pink" => "bg-pink-500",
        "teal" => "bg-teal-500",
        _ => "bg-neutral-400",
    }
}

/// Dezenter Hintergrund + Rand für Termine.
pub fn event_class(color: &str) -> &'static str {
    match color {
        "blue" => "bg-blue-500/15 border-blue-500/60 text-blue-200",
        "green" => "bg-emerald-500/15 border-emerald-500/60 text-emerald-200",
        "orange" => "bg-orange-500/15 border-orange-500/60 text-orange-200",
        "red" => "bg-red-500/15 border-red-500/60 text-red-200",
        "purple" => "bg-violet-500/15 border-violet-500/60 text-violet-200",
        "yellow" => "bg-amber-400/15 border-amber-400/60 text-amber-100",
        "pink" => "bg-pink-500/15 border-pink-500/60 text-pink-200",
        "teal" => "bg-teal-500/15 border-teal-500/60 text-teal-200",
        _ => "bg-neutral-500/15 border-neutral-500/60 text-foreground",
    }
}

pub fn recurrence_hint(r: &str) -> String {
    format!("↻ {}", crate::model::recurrence_label(r))
}

pub fn priority_label(p: i64) -> &'static str {
    match p {
        3 => "Hoch",
        2 => "Mittel",
        1 => "Niedrig",
        _ => "Keine",
    }
}

pub fn priority_class(p: i64) -> &'static str {
    match p {
        3 => "text-red-400",
        2 => "text-orange-400",
        1 => "text-sky-400",
        _ => "text-muted-foreground",
    }
}

// ---------------------------------------------------------------------------
// Schnell-Eingabe: "Steuern machen morgen !! #privat"
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuickAdd {
    pub title: String,
    pub due: Option<NaiveDate>,
    pub priority: i64,
    pub list: Option<String>,
    pub recurrence: Option<String>,
}

fn weekday_from(word: &str) -> Option<Weekday> {
    Some(match word {
        "montag" | "mo" => Weekday::Mon,
        "dienstag" | "di" => Weekday::Tue,
        "mittwoch" | "mi" => Weekday::Wed,
        "donnerstag" | "do" => Weekday::Thu,
        "freitag" | "fr" => Weekday::Fri,
        "samstag" | "sa" => Weekday::Sat,
        "sonntag" | "so" => Weekday::Sun,
        _ => return None,
    })
}

/// Erkennt: heute, morgen, übermorgen, Wochentage, "in 3 tagen", dd.mm.(yyyy),
/// !/!!/!!! (Priorität) und #liste.
pub fn parse_quick_add(input: &str, base: NaiveDate) -> QuickAdd {
    let mut out = QuickAdd::default();
    let words: Vec<&str> = input.split_whitespace().collect();
    let mut keep: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let w = words[i];
        let lower = w.to_lowercase();
        let lw = lower.as_str();
        if out.recurrence.is_none()
            && let Some(r) = match lw {
                "täglich" | "taeglich" => Some("daily"),
                "werktags" => Some("weekdays"),
                "wöchentlich" | "woechentlich" => Some("weekly"),
                "monatlich" => Some("monthly"),
                "jährlich" | "jaehrlich" => Some("yearly"),
                _ => None,
            }
        {
            out.recurrence = Some(r.into());
        } else if out.recurrence.is_none()
            && matches!(lw, "jeden" | "jede" | "jedes")
            && let Some(next) = words.get(i + 1).map(|n| n.to_lowercase())
        {
            let rule = match next.as_str() {
                "tag" => Some("daily"),
                "werktag" => Some("weekdays"),
                "woche" => Some("weekly"),
                "monat" => Some("monthly"),
                "jahr" => Some("yearly"),
                n if weekday_from(n).is_some() && n.len() > 2 => {
                    if out.due.is_none()
                        && let Some(wd) = weekday_from(n)
                    {
                        let mut d = base;
                        while d.weekday() != wd {
                            d += Duration::days(1);
                        }
                        out.due = Some(d);
                    }
                    Some("weekly")
                }
                _ => None,
            };
            if let Some(r) = rule {
                out.recurrence = Some(r.into());
                i += 2;
                continue;
            }
            keep.push(w);
        } else if let Some(tag) = w.strip_prefix('#').filter(|t| !t.is_empty()) {
            out.list = Some(tag.to_string());
        } else if !lw.is_empty() && lw.chars().all(|c| c == '!') && lw.len() <= 3 {
            out.priority = lw.len() as i64;
        } else if out.due.is_none() && lw == "heute" {
            out.due = Some(base);
        } else if out.due.is_none() && lw == "morgen" {
            out.due = Some(base + Duration::days(1));
        } else if out.due.is_none() && (lw == "übermorgen" || lw == "uebermorgen") {
            out.due = Some(base + Duration::days(2));
        } else if out.due.is_none()
            && lw == "in"
            && let Some(n) = words.get(i + 1).and_then(|n| n.parse::<i64>().ok())
            && let Some(unit) = words.get(i + 2).map(|u| u.to_lowercase())
            && (unit.starts_with("tag") || unit.starts_with("woche"))
        {
            let days = if unit.starts_with("woche") { n * 7 } else { n };
            out.due = Some(base + Duration::days(days));
            i += 3;
            continue;
        } else if out.due.is_none()
            && let Some(wd) = weekday_from(lw).filter(|_| lw.len() > 2)
        {
            let mut d = base + Duration::days(1);
            while d.weekday() != wd {
                d += Duration::days(1);
            }
            out.due = Some(d);
        } else if out.due.is_none()
            && let Some(d) = parse_german_date(lw, base)
        {
            out.due = Some(d);
        } else {
            keep.push(w);
        }
        i += 1;
    }
    out.title = keep.join(" ");
    out
}

fn parse_german_date(s: &str, base: NaiveDate) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.trim_end_matches('.').split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    let d: u32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let y: i32 = match parts.get(2) {
        Some(y) if y.len() == 2 => 2000 + y.parse::<i32>().ok()?,
        Some(y) => y.parse().ok()?,
        None => {
            let candidate = NaiveDate::from_ymd_opt(base.year(), m, d)?;
            return Some(if candidate < base { NaiveDate::from_ymd_opt(base.year() + 1, m, d)? } else { candidate });
        }
    };
    NaiveDate::from_ymd_opt(y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_add() {
        let base = NaiveDate::from_ymd_opt(2026, 11, 13).unwrap(); // Freitag
        let q = parse_quick_add("Steuern machen morgen !! #privat", base);
        assert_eq!(q.title, "Steuern machen");
        assert_eq!(q.due, NaiveDate::from_ymd_opt(2026, 11, 14));
        assert_eq!(q.priority, 2);
        assert_eq!(q.list.as_deref(), Some("privat"));

        let q = parse_quick_add("Meeting vorbereiten montag", base);
        assert_eq!(q.due, NaiveDate::from_ymd_opt(2026, 11, 16));
        let q = parse_quick_add("Abo kündigen 3.1.", base);
        assert_eq!(q.due, NaiveDate::from_ymd_opt(2027, 1, 3));
        let q = parse_quick_add("Review in 2 wochen", base);
        assert_eq!(q.due, NaiveDate::from_ymd_opt(2026, 11, 27));
        assert_eq!(q.title, "Review");

        let q = parse_quick_add("Wasser trinken täglich", base);
        assert_eq!(q.recurrence.as_deref(), Some("daily"));
        assert_eq!(q.title, "Wasser trinken");
        let q = parse_quick_add("Müll rausstellen jeden dienstag", base);
        assert_eq!(q.recurrence.as_deref(), Some("weekly"));
        assert_eq!(q.due, NaiveDate::from_ymd_opt(2026, 11, 17));
        assert_eq!(q.title, "Müll rausstellen");
    }

    #[test]
    fn months() {
        assert_eq!(add_months(2026, 12, 1), (2027, 1));
        assert_eq!(add_months(2026, 1, -1), (2025, 12));
        assert_eq!(month_grid(2026, 11)[0], NaiveDate::from_ymd_opt(2026, 10, 26).unwrap());
    }
}
