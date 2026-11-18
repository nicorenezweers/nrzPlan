use dioxus::prelude::*;
use icons::{Pause, Play, RotateCcw, Timer};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Focus,
    Short,
    Long,
}

impl Mode {
    fn secs(self) -> i64 {
        match self {
            Mode::Focus => 25 * 60,
            Mode::Short => 5 * 60,
            Mode::Long => 15 * 60,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Mode::Focus => "Fokus",
            Mode::Short => "Kurze Pause",
            Mode::Long => "Lange Pause",
        }
    }
}

/// Pomodoro-Timer (25/5/15). Läuft nur im Browser.
#[component]
pub fn FocusTimer() -> Element {
    let mut mode = use_signal(|| Mode::Focus);
    let mut remaining = use_signal(|| Mode::Focus.secs());
    let mut running = use_signal(|| false);
    let mut sessions = use_signal(|| 0u32);

    use_future(move || async move {
        loop {
            // Browser-Timer; auf dem Server schlägt eval fehl → Schleife beenden.
            if document::eval("await new Promise(r => setTimeout(r, 1000)); return true;").await.is_err() {
                break;
            }
            if !running() {
                continue;
            }
            let left = remaining() - 1;
            if left > 0 {
                remaining.set(left);
                let m = mode();
                document::eval(&format!(
                    "document.title = '{:02}:{:02} · {} – nrzPlan';",
                    left / 60,
                    left % 60,
                    m.label()
                ));
                continue;
            }
            // Abgelaufen
            running.set(false);
            let finished = mode();
            let next = if finished == Mode::Focus {
                sessions += 1;
                if sessions() % 4 == 0 { Mode::Long } else { Mode::Short }
            } else {
                Mode::Focus
            };
            mode.set(next);
            remaining.set(next.secs());
            document::eval(&format!(
                r#"document.title = 'nrzPlan';
                try {{
                    const c = new (window.AudioContext || window.webkitAudioContext)();
                    [0, 0.25, 0.5].forEach(t => {{
                        const o = c.createOscillator(); const g = c.createGain();
                        o.frequency.value = 880; o.connect(g); g.connect(c.destination);
                        g.gain.setValueAtTime(0.15, c.currentTime + t);
                        g.gain.exponentialRampToValueAtTime(0.001, c.currentTime + t + 0.2);
                        o.start(c.currentTime + t); o.stop(c.currentTime + t + 0.2);
                    }});
                }} catch (e) {{}}
                if (window.Notification && Notification.permission === 'granted') {{
                    new Notification('nrzPlan', {{ body: '{} vorbei – weiter mit: {}' }});
                }}"#,
                finished.label(),
                next.label()
            ));
        }
    });

    let total = mode().secs() as f64;
    let pct = 1.0 - remaining() as f64 / total;
    let r = 52.0;
    let circ = 2.0 * std::f64::consts::PI * r;
    let tab = |active: bool| {
        if active { "rounded-md bg-accent px-2.5 py-1 text-xs font-medium" } else { "rounded-md px-2.5 py-1 text-xs text-muted-foreground hover:text-foreground" }
    };
    let mut select = move |m: Mode| {
        running.set(false);
        mode.set(m);
        remaining.set(m.secs());
        document::eval("document.title = 'nrzPlan';");
    };

    rsx! {
        div { class: "flex flex-col items-center gap-4 rounded-xl border bg-card/40 p-5",
            div { class: "flex w-full items-center justify-between",
                span { class: "flex items-center gap-2 text-sm font-medium",
                    Timer { class: "size-4 text-muted-foreground" }
                    "Fokus-Timer"
                }
                span { class: "text-xs text-muted-foreground", "{sessions()} Sessions heute" }
            }
            div { class: "flex rounded-lg border p-0.5",
                button { class: tab(mode() == Mode::Focus), onclick: move |_| select(Mode::Focus), "Fokus" }
                button { class: tab(mode() == Mode::Short), onclick: move |_| select(Mode::Short), "Pause" }
                button { class: tab(mode() == Mode::Long), onclick: move |_| select(Mode::Long), "Lang" }
            }
            div { class: "relative size-36",
                svg { class: "size-36 -rotate-90", view_box: "0 0 120 120",
                    circle { cx: "60", cy: "60", r: "{r}", fill: "none", stroke: "currentColor", stroke_opacity: "0.12", stroke_width: "6" }
                    circle {
                        cx: "60",
                        cy: "60",
                        r: "{r}",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "6",
                        stroke_linecap: "round",
                        stroke_dasharray: "{circ}",
                        stroke_dashoffset: "{circ * (1.0 - pct)}",
                        class: "transition-[stroke-dashoffset] duration-1000 ease-linear",
                    }
                }
                div { class: "absolute inset-0 flex flex-col items-center justify-center",
                    span { class: "font-mono text-3xl font-semibold tabular-nums", "{remaining() / 60:02}:{remaining() % 60:02}" }
                    span { class: "text-xs text-muted-foreground", "{mode().label()}" }
                }
            }
            div { class: "flex items-center gap-2",
                button {
                    class: "flex h-9 items-center gap-2 rounded-full bg-primary px-5 text-sm font-medium text-primary-foreground hover:bg-primary/90",
                    onclick: move |_| {
                        running.toggle();
                        // Einmalig Erlaubnis für Desktop-Benachrichtigungen anfragen
                        document::eval("if (window.Notification && Notification.permission === 'default') Notification.requestPermission();");
                    },
                    if running() { Pause { class: "size-4" } "Pause" } else { Play { class: "size-4" } "Start" }
                }
                button {
                    class: "flex size-9 items-center justify-center rounded-full border text-muted-foreground hover:text-foreground",
                    title: "Zurücksetzen",
                    onclick: move |_| select(mode()),
                    RotateCcw { class: "size-4" }
                }
            }
        }
    }
}
