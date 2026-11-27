//! Generation Queue — rendered entirely from live server data.
//!
//! Rows come from the real `GET /queue` snapshot (running + pending) joined
//! with `GET /history` (completed / failed), correlated back to storyboard
//! shots via prompt_id. Progress, elapsed time and the task log are driven by
//! the live WebSocket feed; there is no mock fallback data.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use egui::{Align2, Color32, FontId, FontFamily, Rect, Sense, Stroke, StrokeKind};
use crate::app::{first_output_filename, H3App};
use crate::theme::color;
use crate::ui_state::QueueFilter;
use crate::widgets;

#[derive(Clone, Copy, PartialEq)]
enum RowStatus {
    Running,
    Queued,
    Done,
    Failed,
}

struct QueueRow {
    id: String,
    label: String,
    mode: String,
    status: RowStatus,
    progress: f32,
    stage: String,
    elapsed: String,
    gpu: String,
    output: Option<String>,
    #[allow(dead_code)]
    prompt_id: String,
}

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    use crate::i18n::{tr, tf};
    let lang = app.lang();
    let running = app
        .queue
        .get("queue_running")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let pending = app
        .queue
        .get("queue_pending")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let completed = app.completed_count();

    // Filter chips + summary
    ui.horizontal(|ui| {
        let f = app.ui.queue_filter;
        if chip(ui, tr(lang, "All"), widgets::Tone::Gray, f == QueueFilter::All) {
            app.ui.queue_filter = QueueFilter::All;
        }
        ui.add_space(4.0);
        if chip(ui, &tf(lang, "{} Running", &[&running]), widgets::Tone::Indigo, f == QueueFilter::Running) {
            app.ui.queue_filter = QueueFilter::Running;
        }
        ui.add_space(4.0);
        if chip(ui, &tf(lang, "{} Queued", &[&pending]), widgets::Tone::Gray, f == QueueFilter::Queued) {
            app.ui.queue_filter = QueueFilter::Queued;
        }
        ui.add_space(4.0);
        if chip(ui, &tf(lang, "{} Completed", &[&completed]), widgets::Tone::Teal, f == QueueFilter::Completed) {
            app.ui.queue_filter = QueueFilter::Completed;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let txt = if app.ws_connected {
                tr(lang, "● live feed")
            } else if app.connect_ok {
                tr(lang, "○ polling")
            } else {
                tr(lang, "○ offline")
            };
            let c = if app.ws_connected { color::teal() } else { color::text3() };
            ui.colored_label(c, txt);
        });
    });
    ui.add_space(10.0);

    let mut rows = live_rows(app);
    let f = app.ui.queue_filter;
    rows.retain(|r| match f {
        QueueFilter::All => true,
        QueueFilter::Running => r.status == RowStatus::Running,
        QueueFilter::Queued => r.status == RowStatus::Queued,
        QueueFilter::Completed => r.status == RowStatus::Done || r.status == RowStatus::Failed,
    });

    let visible = rows.len().min(12);
    let table_h = (36.0 + visible as f32 * 62.0).max(220.0).min(520.0);
    let (table, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), table_h), egui::Sense::hover());
    ui.painter().rect_filled(table, 12.0f32, color::panel());
    ui.painter()
        .rect_stroke(table, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);

    // header
    let header_rect = Rect::from_min_size(
        egui::pos2(table.left(), table.top()),
        egui::vec2(table.width(), 36.0),
    );
    ui.painter().rect_filled(header_rect, 12.0f32, color::card());
    ui.painter().text(
        egui::pos2(table.left() + 20.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        tr(lang, "ID"),
        FontId::new(10.5, FontFamily::Proportional),
        color::text2(),
    );
    ui.painter().text(
        egui::pos2(table.left() + 84.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        tr(lang, "SHOT / JOB"),
        FontId::new(10.5, FontFamily::Proportional),
        color::text2(),
    );
    ui.painter().text(
        egui::pos2(table.left() + 430.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        tr(lang, "MODE"),
        FontId::new(10.5, FontFamily::Proportional),
        color::text2(),
    );
    ui.painter().text(
        egui::pos2(table.left() + 560.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        tr(lang, "STATUS"),
        FontId::new(10.5, FontFamily::Proportional),
        color::text2(),
    );
    ui.painter().text(
        egui::pos2(table.left() + 730.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        tr(lang, "PROGRESS"),
        FontId::new(10.5, FontFamily::Proportional),
        color::text2(),
    );
    ui.painter().text(
        egui::pos2(table.left() + 1070.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        tr(lang, "ELAPSED"),
        FontId::new(10.5, FontFamily::Proportional),
        color::text2(),
    );
    ui.painter().text(
        egui::pos2(table.left() + 1240.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        tr(lang, "GPU"),
        FontId::new(10.5, FontFamily::Proportional),
        color::text2(),
    );
    ui.painter().text(
        egui::pos2(table.left() + 1330.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        tr(lang, "ACTIONS"),
        FontId::new(10.5, FontFamily::Proportional),
        color::text2(),
    );

    if rows.is_empty() {
        ui.painter().text(
            table.center(),
            Align2::CENTER_CENTER,
            tr(lang, "No jobs — generate a shot to queue it here."),
            FontId::new(12.0, FontFamily::Proportional),
            color::text3(),
        );
    }

    let mut y = table.top() + 36.0;
    for (idx, row) in rows.iter().enumerate().take(12) {
        let row_h = 62.0;
        let row_rect = Rect::from_min_size(egui::pos2(table.left(), y), egui::vec2(table.width(), row_h));
        if idx % 2 == 1 {
            ui.painter().rect_filled(
                row_rect,
                0.0f32,
                Color32::from_rgba_premultiplied(0x1A, 0x1D, 0x22, 120),
            );
        }
        ui.painter().rect_stroke(
            Rect::from_min_size(egui::pos2(row_rect.left(), row_rect.bottom() - 1.0), egui::vec2(row_rect.width(), 1.0)),
            0.0f32,
            Stroke::new(1.0, color::border()),
            StrokeKind::Middle,
        );

        let tone = row_tone(row.status);
        let status_txt = row_label(lang, row.status);
        ui.painter().text(
            egui::pos2(row_rect.left() + 20.0, row_rect.center().y - 7.0),
            Align2::LEFT_CENTER,
            &row.id,
            FontId::new(11.5, FontFamily::Monospace),
            color::text(),
        );
        if let Some(out) = row.output_text() {
            ui.painter().text(
                egui::pos2(row_rect.left() + 20.0, row_rect.center().y + 11.0),
                Align2::LEFT_CENTER,
                &out,
                FontId::new(9.0, FontFamily::Monospace),
                color::text3(),
            );
        }
        ui.painter().text(
            egui::pos2(row_rect.left() + 84.0, row_rect.center().y - 5.0),
            Align2::LEFT_CENTER,
            &row.label,
            FontId::new(11.5, FontFamily::Proportional),
            color::text(),
        );
        ui.painter().text(
            egui::pos2(row_rect.left() + 430.0, row_rect.center().y),
            Align2::LEFT_CENTER,
            &row.mode,
            FontId::new(11.0, FontFamily::Proportional),
            color::text2(),
        );

        let chip_rect = Rect::from_min_size(
            egui::pos2(row_rect.left() + 560.0, row_rect.center().y - 12.0),
            egui::vec2(104.0, 24.0),
        );
        ui.painter().rect_filled(chip_rect, 12.0f32, tone.dim());
        ui.painter().rect_stroke(
            chip_rect,
            12.0f32,
            Stroke::new(1.0, tone.accent().linear_multiply(0.45)),
            StrokeKind::Middle,
        );
        ui.painter().text(
            chip_rect.center(),
            Align2::CENTER_CENTER,
            status_txt,
            FontId::new(10.5, FontFamily::Proportional),
            tone.accent(),
        );

        let bar_rect = Rect::from_min_size(
            egui::pos2(row_rect.left() + 730.0, row_rect.center().y - 5.0),
            egui::vec2(150.0, 10.0),
        );
        widgets::progress_bar(ui, bar_rect, row.progress, tone.accent(), 10.0);
        ui.painter().text(
            egui::pos2(row_rect.left() + 730.0, bar_rect.bottom() + 8.0),
            Align2::LEFT_CENTER,
            &row.stage,
            FontId::new(9.0, FontFamily::Proportional),
            color::text3(),
        );

        ui.painter().text(
            egui::pos2(row_rect.left() + 1070.0, row_rect.center().y),
            Align2::LEFT_CENTER,
            &row.elapsed,
            FontId::new(11.0, FontFamily::Monospace),
            color::text2(),
        );
        ui.painter().text(
            egui::pos2(row_rect.left() + 1240.0, row_rect.center().y),
            Align2::LEFT_CENTER,
            &row.gpu,
            FontId::new(10.5, FontFamily::Proportional),
            color::text2(),
        );

        let btn_rect = Rect::from_min_size(
            egui::pos2(row_rect.left() + 1330.0, row_rect.center().y - 14.0),
            egui::vec2(72.0, 28.0),
        );
        let btn_resp = ui.interact(btn_rect, ui.id().with(("queue-btn", idx)), egui::Sense::click());
        ui.painter().rect_filled(
            btn_rect,
            6.0f32,
            if btn_resp.hovered() { color::card() } else { Color32::TRANSPARENT },
        );
        ui.painter()
            .rect_stroke(btn_rect, 6.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            tr(lang, "Logs"),
            FontId::new(10.0, FontFamily::Proportional),
            color::text(),
        );
        if btn_resp.clicked() {
            app.set_toast(tf(lang, "Showing logs for {} · {}", &[&row.id, &row.label]));
        }

        y += row_h;
    }

    ui.add_space(12.0);

    // Task log — real lines from the WebSocket feed.
    let (log, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 176.0), egui::Sense::hover());
    ui.painter().rect_filled(log, 12.0f32, color::panel());
    ui.painter()
        .rect_stroke(log, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    let feed = if app.ws_connected {
        tr(lang, "TASK LOG — live feed")
    } else {
        tr(lang, "TASK LOG — feed offline")
    };
    ui.painter().text(
        egui::pos2(log.left() + 20.0, log.top() + 24.0),
        Align2::LEFT_CENTER,
        feed,
        FontId::new(10.5, FontFamily::Proportional),
        color::text(),
    );

    let total = app.ws_log.len();
    let start = total.saturating_sub(12);
    let lines: Vec<&String> = app
        .ws_log
        .iter()
        .skip(start)
        .map(|(_, s)| s)
        .collect();
    if lines.is_empty() {
        ui.painter().text(
            egui::pos2(log.left() + 24.0, log.top() + 52.0),
            Align2::LEFT_CENTER,
            tr(lang, "No live activity yet. Generate a shot to see execution logs here."),
            FontId::new(10.5, FontFamily::Monospace),
            color::text3(),
        );
    } else {
        for (i, line) in lines.iter().enumerate() {
            ui.painter().text(
                egui::pos2(log.left() + 24.0, log.top() + 48.0 + i as f32 * 21.0),
                Align2::LEFT_CENTER,
                line.as_str(),
                FontId::new(10.0, FontFamily::Monospace),
                color::teal().linear_multiply(0.9),
            );
        }
    }
    if total > 12 {
        ui.painter().text(
            egui::pos2(log.right() - 20.0, log.top() + 24.0),
            Align2::RIGHT_CENTER,
            &tf(lang, "showing last {} of {} lines", &[&12usize, &total]),
            FontId::new(9.0, FontFamily::Proportional),
            color::text3(),
        );
    }
}

impl QueueRow {
    fn output_text(&self) -> Option<String> {
        self.output.as_ref().map(|o| {
            let short: String = o.chars().rev().take(30).collect::<String>().chars().rev().collect();
            format!("↳ {}", short)
        })
    }
}

fn row_tone(status: RowStatus) -> widgets::Tone {
    match status {
        RowStatus::Running => widgets::Tone::Indigo,
        RowStatus::Done => widgets::Tone::Teal,
        RowStatus::Failed => widgets::Tone::Red,
        RowStatus::Queued => widgets::Tone::Gray,
    }
}

fn row_label(lang: crate::i18n::Lang, status: RowStatus) -> &'static str {
    match status {
        RowStatus::Running => crate::i18n::tr(lang, "Running"),
        RowStatus::Queued => crate::i18n::tr(lang, "Queued"),
        RowStatus::Done => crate::i18n::tr(lang, "Done"),
        RowStatus::Failed => crate::i18n::tr(lang, "Failed"),
    }
}

fn short_id(prompt_id: &str) -> String {
    prompt_id.chars().take(8).collect()
}

fn fmt_elapsed(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

fn shot_meta(app: &H3App, pid: &str, prompt: Option<&serde_json::Value>) -> (String, String) {
    if let Some((_, _, shot)) = app.local.shot_by_prompt(pid) {
        (format!("Shot {:02} · {}", shot.number, shot.title), shot.mode.clone())
    } else {
        let mode = prompt.and_then(detect_mode).unwrap_or_else(|| "—".to_string());
        (format!("job {}", short_id(pid)), mode)
    }
}

fn detect_mode(prompt: &serde_json::Value) -> Option<String> {
    let obj = prompt.as_object()?;
    for v in obj.values() {
        let ct = v.get("class_type").and_then(|x| x.as_str()).unwrap_or("");
        if ct.contains("MiniMaxH3") || ct.contains("H3Pipeline") {
            if let Some(m) = v.get("inputs").and_then(|i| i.get("mode")).and_then(|x| x.as_str()) {
                return Some(m.to_string());
            }
        }
    }
    None
}

fn active_row(app: &H3App, pid: &str, status: RowStatus, now: Instant, pos: usize) -> QueueRow {
    let (label, mode) = shot_meta(app, pid, None);
    let progress = if status == RowStatus::Running {
        app.ws_progress
            .get(pid)
            .map(|p| if p.max > 0.0 { (p.value / p.max) as f32 } else { 0.0 })
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let stage = match status {
        RowStatus::Running => {
            if let Some(p) = app.ws_progress.get(pid) {
                if p.max > 0.0 {
                    let pct = (p.value / p.max * 100.0).round() as u32;
                    let node = app.ws_node.get(pid).map(|n| format!(" · node {}", n)).unwrap_or_default();
                    format!("{}%{}", pct, node)
                } else {
                    "running…".to_string()
                }
            } else {
                "running…".to_string()
            }
        }
        RowStatus::Queued => format!("queued · position #{}", pos),
        _ => String::new(),
    };
    let elapsed = if status == RowStatus::Running {
        app.job_started_at
            .get(pid)
            .map(|s| fmt_elapsed(now - *s))
            .unwrap_or_else(|| "—".to_string())
    } else {
        format!("#{}", pos)
    };
    let gpu = if status == RowStatus::Running {
        "GPU 0".to_string()
    } else {
        "—".to_string()
    };
    QueueRow {
        id: short_id(pid),
        label,
        mode,
        status,
        progress,
        stage,
        elapsed,
        gpu,
        output: None,
        prompt_id: pid.to_string(),
    }
}

fn history_row(
    app: &H3App,
    pid: &str,
    entry: &serde_json::Value,
    status: RowStatus,
) -> QueueRow {
    let prompt = entry.get("prompt");
    let (label, mode) = shot_meta(app, pid, prompt);
    let progress = if status == RowStatus::Done { 1.0 } else { 0.0 };
    let stage = match status {
        RowStatus::Done => "complete".to_string(),
        RowStatus::Failed => "failed".to_string(),
        _ => String::new(),
    };
    let elapsed = match (app.job_started_at.get(pid), app.job_completed_at.get(pid)) {
        (Some(s), Some(e)) if *e >= *s => fmt_elapsed(*e - *s),
        _ => "—".to_string(),
    };
    // Prefer the shot's recorded output, else parse the history outputs.
    let output = app
        .local
        .shot_by_prompt(pid)
        .and_then(|(_, _, s)| s.output.clone())
        .or_else(|| first_output_filename(&entry.get("outputs").cloned().unwrap_or_default()));
    QueueRow {
        id: short_id(pid),
        label,
        mode,
        status,
        progress,
        stage,
        elapsed,
        gpu: "—".to_string(),
        output,
        prompt_id: pid.to_string(),
    }
}

fn live_rows(app: &H3App) -> Vec<QueueRow> {
    let now = Instant::now();
    let mut rows = Vec::new();
    let mut active = HashSet::new();

    let arr = |key: &str| {
        app.queue
            .get(key)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };
    let running = arr("queue_running");
    let pending = arr("queue_pending");

    for item in &running {
        if let Some(pid) = item.get("prompt_id").and_then(|x| x.as_str()) {
            active.insert(pid.to_string());
            rows.push(active_row(app, pid, RowStatus::Running, now, 0));
        }
    }
    for (i, item) in pending.iter().enumerate() {
        if let Some(pid) = item.get("prompt_id").and_then(|x| x.as_str()) {
            active.insert(pid.to_string());
            rows.push(active_row(app, pid, RowStatus::Queued, now, i + 1));
        }
    }

    if let Some(map) = app.history_recent.as_object() {
        let mut entries: Vec<(&String, &serde_json::Value)> = map.iter().collect();
        entries.sort_by(|a, b| b.0.cmp(a.0));
        for (pid, entry) in entries {
            if active.contains(pid.as_str()) {
                continue;
            }
            let status = match entry.get("status") {
                Some(serde_json::Value::String(s)) if s == "Completed" => RowStatus::Done,
                Some(serde_json::Value::String(s)) if s == "Failed" => RowStatus::Failed,
                Some(serde_json::Value::Object(_)) => RowStatus::Failed,
                _ => continue,
            };
            rows.push(history_row(app, pid, entry, status));
            if rows.len() >= 80 {
                break;
            }
        }
    }
    rows
}

fn chip(ui: &mut egui::Ui, text: &str, tone: widgets::Tone, active: bool) -> bool {
    let w = widgets::est_text_width(text, 11.0) + 26.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w.max(90.0), 26.0), Sense::click());
    let hov = resp.hovered() && !active;
    ui.painter().rect_filled(
        rect,
        13.0f32,
        if active || hov {
            tone.dim().linear_multiply(1.3)
        } else {
            tone.dim()
        },
    );
    ui.painter().rect_stroke(
        rect,
        13.0f32,
        Stroke::new(
            if active { 1.4 } else { 1.0 },
            tone.accent().linear_multiply(if active { 0.9 } else { 0.45 }),
        ),
        StrokeKind::Middle,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::new(11.0, FontFamily::Proportional),
        tone.accent(),
    );
    resp.clicked()
}
