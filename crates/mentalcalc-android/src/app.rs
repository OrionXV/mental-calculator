//! egui drawing. Screens only read `AppState` and push `Action`s, which are
//! applied after the frame is drawn.

use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, FontId, RichText, TextStyle, Ui, Vec2};
use mentalcalc_core::{Difficulty, Operation, Phase};
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::state::{
    Action, AppState, Dialog, Notice, Screen, grade_for, percent_correct, total_tasks,
};
use crate::storage::{self, Storage};
use crate::strings::{self, Key, Language, text};

const KEY_HEIGHT: f32 = 60.0;
const BUTTON_HEIGHT: f32 = 56.0;
/// Font size for keys whose label is a word (Стереть, Ответ) so they fit a key's width.
const KEY_WORD_SIZE: f32 = 22.0;
const CORRECT: Color32 = Color32::from_rgb(46, 125, 50);
const INCORRECT: Color32 = Color32::from_rgb(198, 40, 40);
const MUTED: Color32 = Color32::from_rgb(120, 120, 120);

pub struct App {
    state: AppState,
    /// Monotonic time of the previous frame, used for the session clock.
    last_frame: Option<Instant>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, storage: Storage, screen: Screen) -> Self {
        configure_style(&cc.egui_ctx);
        let mut state = AppState::new(storage, StdRng::from_os_rng());
        state.open_preview(screen);
        Self {
            state,
            last_frame: None,
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // TODO(android back button): not handled yet. It needs a device test. When added,
        // intercept the Back key here and map it to `Action::Open` of the previous screen.
        // TODO(edge-to-edge): window insets are not applied yet. Needs a device test.
        let now = Instant::now();
        let gap = self
            .last_frame
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        self.last_frame = Some(now);
        self.state.advance_frame(gap, storage::now());
        self.state
            .set_window_focused(ctx.input(|input| input.focused));
        if self.state.screen == Screen::Session || self.state.feedback.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let mut actions = Vec::new();
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).inner_margin(16.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| draw_screen(ui, &self.state, &mut actions));
            });
        for action in actions {
            self.state.apply(action);
        }
    }
}

fn configure_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(30.0)),
            (TextStyle::Body, FontId::proportional(20.0)),
            (TextStyle::Button, FontId::proportional(22.0)),
            (TextStyle::Monospace, FontId::monospace(20.0)),
            (TextStyle::Small, FontId::proportional(15.0)),
        ]
        .into();
        style.spacing.item_spacing = Vec2::new(12.0, 12.0);
        style.spacing.button_padding = Vec2::new(12.0, 8.0);
    });
}

fn draw_screen(ui: &mut Ui, state: &AppState, out: &mut Vec<Action>) {
    let lang = state.language;
    match state.screen {
        Screen::Main => main_screen(ui, state, lang, out),
        Screen::SessionSettings => session_settings_screen(ui, state, lang, out),
        Screen::Session => session_screen(ui, state, lang, out),
        Screen::Result => result_screen(ui, state, lang, out),
        Screen::Archive => archive_screen(ui, state, lang, out),
        Screen::Settings => settings_screen(ui, state, lang, out),
    }
    if let Some(dialog) = state.dialog {
        dialog_window(ui.ctx(), dialog, lang, out);
    }
}

fn main_screen(ui: &mut Ui, state: &AppState, lang: Language, out: &mut Vec<Action>) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| ui.heading(text(lang, Key::AppName)));
    ui.add_space(24.0);
    let full = Vec2::new(ui.available_width(), BUTTON_HEIGHT + 8.0);
    if button(ui, text(lang, Key::Start), full) {
        out.push(Action::StartSession);
    }
    if button(ui, text(lang, Key::Archive), full) {
        out.push(Action::Open(Screen::Archive));
    }
    if button(ui, text(lang, Key::SessionSettings), full) {
        out.push(Action::Open(Screen::SessionSettings));
    }
    if button(ui, text(lang, Key::Settings), full) {
        out.push(Action::Open(Screen::Settings));
    }
    notice_line(ui, state.notice.as_ref(), lang);
}

fn session_settings_screen(ui: &mut Ui, state: &AppState, lang: Language, out: &mut Vec<Action>) {
    header(ui, lang, Key::SessionSettings, Some(Screen::Main), out);
    let settings = &state.settings;

    ui.label(RichText::new(text(lang, Key::Duration)).strong());
    ui.horizontal(|ui| {
        let side = Vec2::new(BUTTON_HEIGHT * 1.4, BUTTON_HEIGHT);
        if button(ui, "-", side) {
            out.push(Action::AdjustDuration(-1));
        }
        let label = strings::minutes(lang, settings.duration_minutes);
        ui.add_sized(
            [ui.available_width() - side.x - 12.0, BUTTON_HEIGHT],
            egui::Label::new(RichText::new(label).size(26.0).strong()),
        );
        if button(ui, "+", side) {
            out.push(Action::AdjustDuration(1));
        }
    });

    ui.add_space(8.0);
    ui.label(RichText::new(text(lang, Key::Difficulty)).strong());
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Medium,
        Difficulty::Hard,
        Difficulty::Expert,
    ] {
        let selected = settings.difficulty == difficulty;
        let response = ui.radio(selected, strings::difficulty(lang, difficulty));
        if response.clicked() {
            out.push(Action::SetDifficulty(difficulty));
        }
    }

    ui.add_space(8.0);
    ui.label(RichText::new(text(lang, Key::Operations)).strong());
    for op in Operation::ALL {
        let mut on = settings.operations.contains(op);
        let response = ui.add(egui::Checkbox::new(&mut on, strings::operation(lang, op)));
        if response.clicked() {
            out.push(Action::ToggleOperation(op));
        }
    }
    notice_line(ui, state.notice.as_ref(), lang);
}

fn session_screen(ui: &mut Ui, state: &AppState, lang: Language, out: &mut Vec<Action>) {
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let phase = session.phase();
    let running = phase == Phase::Running;
    let paused = phase == Phase::Paused;
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(text(lang, Key::TimeLeft))
                .size(16.0)
                .color(MUTED),
        );
        ui.label(
            RichText::new(format_remaining(session.remaining()))
                .size(52.0)
                .strong(),
        );
        ui.horizontal(|ui| {
            let half = (ui.available_width() - 12.0) / 2.0;
            ui.add_sized(
                [half, 28.0],
                egui::Label::new(
                    RichText::new(format!("{}: {}", text(lang, Key::Correct), state.correct))
                        .color(CORRECT)
                        .size(18.0),
                ),
            );
            ui.add_sized(
                [half, 28.0],
                egui::Label::new(
                    RichText::new(format!(
                        "{}: {}",
                        text(lang, Key::Incorrect),
                        state.incorrect
                    ))
                    .color(INCORRECT)
                    .size(18.0),
                ),
            );
        });
    });

    ui.add_space(8.0);
    ui.vertical_centered(|ui| {
        let problem = if paused {
            text(lang, Key::Paused).to_owned()
        } else {
            format!("{} =", session.current_task())
        };
        ui.add_sized(
            [ui.available_width(), 72.0],
            egui::Label::new(RichText::new(problem).size(44.0).strong()),
        );
        let answer = if state.input.is_empty() {
            "?".to_owned()
        } else {
            state.input.clone()
        };
        ui.add_sized(
            [ui.available_width(), 56.0],
            egui::Label::new(RichText::new(answer).size(40.0)),
        );
        let feedback = state.feedback.map(|feedback| {
            strings::skipped_task(lang, &feedback.task.to_string(), feedback.task.answer)
        });
        ui.add_sized(
            [ui.available_width(), 28.0],
            egui::Label::new(
                RichText::new(feedback.unwrap_or_default())
                    .color(INCORRECT)
                    .size(18.0),
            ),
        );
    });

    ui.horizontal(|ui| {
        let half = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
        let size = Vec2::new(half, BUTTON_HEIGHT);
        let pause_label = if paused {
            text(lang, Key::Resume)
        } else {
            text(lang, Key::Pause)
        };
        if button(ui, pause_label, size) {
            out.push(Action::TogglePause);
        }
        if button(ui, text(lang, Key::End), size) {
            out.push(Action::AskEndSession);
        }
    });

    let spacing = ui.spacing().item_spacing.x;
    let key_size = Vec2::new((ui.available_width() - 2.0 * spacing) / 3.0, KEY_HEIGHT);
    for row in [[1u8, 2, 3], [4, 5, 6], [7, 8, 9]] {
        ui.horizontal(|ui| {
            for digit in row {
                if keypad(ui, running, &digit.to_string(), key_size) {
                    out.push(Action::Digit(digit));
                }
            }
        });
    }
    ui.horizontal(|ui| {
        if keypad(ui, running, text(lang, Key::Delete), key_size) {
            out.push(Action::Backspace);
        }
        if keypad(ui, running, "0", key_size) {
            out.push(Action::Digit(0));
        }
        if keypad(ui, running, text(lang, Key::Answer), key_size) {
            out.push(Action::Submit);
        }
    });
}

fn result_screen(ui: &mut Ui, state: &AppState, lang: Language, out: &mut Vec<Action>) {
    header(ui, lang, Key::ResultTitle, Some(Screen::Main), out);
    let Some(summary) = state.result.as_ref() else {
        return;
    };
    egui::Grid::new("result_grid")
        .num_columns(2)
        .spacing([24.0, 14.0])
        .show(ui, |ui| {
            let row = |ui: &mut Ui, label: Key, value: String| {
                ui.label(RichText::new(text(lang, label)).size(18.0));
                ui.label(RichText::new(value).size(22.0).strong());
                ui.end_row();
            };
            row(
                ui,
                Key::ResultGrade,
                strings::grade(lang, summary.grade).to_owned(),
            );
            row(
                ui,
                Key::ResultPercent,
                format!("{}%", summary.percent_correct),
            );
            row(
                ui,
                Key::ResultSpeed,
                format!("{} {}", summary.speed, text(lang, Key::SpeedUnit)),
            );
            row(ui, Key::ResultTotal, summary.tasks.to_string());
            row(ui, Key::ResultCorrect, summary.correct.to_string());
            row(ui, Key::ResultIncorrect, summary.incorrect.to_string());
            row(ui, Key::Duration, strings::minutes(lang, summary.minutes));
        });
    ui.add_space(16.0);
    if button(
        ui,
        text(lang, Key::Back),
        Vec2::new(ui.available_width(), BUTTON_HEIGHT),
    ) {
        out.push(Action::Open(Screen::Main));
    }
    notice_line(ui, state.notice.as_ref(), lang);
}

fn archive_screen(ui: &mut Ui, state: &AppState, lang: Language, out: &mut Vec<Action>) {
    header(ui, lang, Key::Archive, Some(Screen::Main), out);
    if state.records.is_empty() {
        ui.label(text(lang, Key::ArchiveEmpty));
    } else {
        let sessions = state.records.len();
        let solved = total_tasks(&state.records);
        ui.label(format!(
            "{}: {sessions}    {}: {solved}",
            text(lang, Key::ArchiveSessions),
            text(lang, Key::ArchiveSolved)
        ));
        ui.add_space(4.0);
        for record in &state.records {
            let grade = grade_for(record.tasks, record.correct);
            let percent = percent_correct(record.tasks, record.correct);
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(format_date(&record.date))
                        .size(16.0)
                        .color(MUTED),
                );
                ui.label(
                    RichText::new(format!(
                        "{}/{}  ({}%)  {}",
                        record.correct,
                        record.tasks,
                        percent,
                        strings::grade(lang, grade)
                    ))
                    .size(20.0)
                    .strong(),
                );
                let speed = record.tasks / record.minutes.max(1);
                ui.label(
                    RichText::new(format!(
                        "{} · {} · {speed} {}",
                        strings::minutes(lang, record.minutes),
                        strings::difficulty(lang, record.difficulty),
                        text(lang, Key::SpeedUnit)
                    ))
                    .size(16.0),
                );
            });
        }
    }
    ui.add_space(8.0);
    notice_line(ui, state.notice.as_ref(), lang);
    if state.skipped_lines > 0 {
        ui.label(
            RichText::new(strings::skipped_lines(lang, state.skipped_lines))
                .size(15.0)
                .color(MUTED),
        );
    }
    ui.add_space(8.0);
    let full = Vec2::new(ui.available_width(), BUTTON_HEIGHT);
    let clear = ui.add_enabled(
        !state.records.is_empty(),
        egui::Button::new(text(lang, Key::ArchiveClear)).min_size(full),
    );
    if clear.clicked() {
        out.push(Action::AskClearArchive);
    }
}

fn settings_screen(ui: &mut Ui, state: &AppState, lang: Language, out: &mut Vec<Action>) {
    header(ui, lang, Key::Settings, Some(Screen::Main), out);
    ui.label(RichText::new(text(lang, Key::Language)).strong());
    let full = Vec2::new(ui.available_width(), BUTTON_HEIGHT);
    for option in Language::ALL {
        let selected = state.language == option;
        let response =
            ui.add(egui::Button::selectable(selected, option.native_name()).min_size(full));
        if response.clicked() {
            out.push(Action::SetLanguage(option));
        }
    }
    notice_line(ui, state.notice.as_ref(), lang);
}

fn dialog_window(ctx: &egui::Context, dialog: Dialog, lang: Language, out: &mut Vec<Action>) {
    let (title, message, confirm_key, confirm_action) = match dialog {
        Dialog::EndSession => (
            Key::EndTitle,
            Key::EndMessage,
            Key::End,
            Action::ConfirmEndSession,
        ),
        Dialog::ClearArchive => (
            Key::ArchiveClearTitle,
            Key::ArchiveClearMessage,
            Key::Clear,
            Action::ConfirmClearArchive,
        ),
    };
    egui::Window::new(text(lang, title))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_max_width(320.0);
            ui.label(text(lang, message));
            ui.add_space(12.0);
            let half = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
            let size = Vec2::new(half, BUTTON_HEIGHT);
            ui.horizontal(|ui| {
                if button(ui, text(lang, Key::Cancel), size) {
                    out.push(Action::CancelDialog);
                }
                if button(ui, text(lang, confirm_key), size) {
                    out.push(confirm_action);
                }
            });
        });
}

fn header(ui: &mut Ui, lang: Language, title: Key, back: Option<Screen>, out: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if let Some(screen) = back
            && button(ui, text(lang, Key::Back), Vec2::new(0.0, BUTTON_HEIGHT))
        {
            out.push(Action::Open(screen));
        }
        ui.heading(text(lang, title));
    });
    ui.add_space(8.0);
}

fn button(ui: &mut Ui, label: impl Into<RichText>, size: Vec2) -> bool {
    ui.add(egui::Button::new(label.into()).min_size(size))
        .clicked()
}

/// A keypad key of exactly `size`. Word labels use a smaller font so the text
/// fits; a wider label would otherwise grow its button past the keypad grid.
fn keypad(ui: &mut Ui, enabled: bool, label: &str, size: Vec2) -> bool {
    let font_size = if label.chars().count() > 1 {
        KEY_WORD_SIZE
    } else {
        28.0
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).size(font_size).strong()).min_size(size),
    )
    .clicked()
}

fn notice_line(ui: &mut Ui, notice: Option<&Notice>, lang: Language) {
    let Some(notice) = notice else {
        return;
    };
    let message = match notice {
        Notice::ConfigureFirst => text(lang, Key::ConfigureFirst).to_owned(),
        Notice::Failure(detail) => strings::storage_failure(lang, detail),
    };
    ui.add_space(8.0);
    ui.label(RichText::new(message).color(INCORRECT));
}

/// `m:ss`, rounding up so the display shows 0:00 only when time is up.
pub fn format_remaining(remaining: Duration) -> String {
    let secs = remaining.as_secs_f64().ceil() as u64;
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn format_date(t: &mentalcalc_core::archive::Timestamp) -> String {
    format!(
        "{:04}/{:02}/{:02}  {:02}:{:02}",
        t.year, t.month, t.day, t.hour, t.minute
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_time_rounds_up_to_whole_seconds() {
        assert_eq!(format_remaining(Duration::from_secs(300)), "5:00");
        assert_eq!(format_remaining(Duration::from_millis(59_100)), "1:00");
        assert_eq!(format_remaining(Duration::from_secs(9)), "0:09");
        assert_eq!(format_remaining(Duration::ZERO), "0:00");
    }
}
