//! Screen state machine. Pure logic with no egui types, so the transitions can
//! be tested directly. The UI reads this state and sends `Action`s back.

use std::time::Duration;

use mentalcalc_core::archive::Timestamp;
use mentalcalc_core::session::{MAX_ANSWER_DIGITS, Outcome};
use mentalcalc_core::settings::{MAX_DURATION_MINUTES, MIN_DURATION_MINUTES};
use mentalcalc_core::{
    ArchiveRecord, Difficulty, Grade, Operation, Phase, Session, SessionSummary, Settings, Task,
};
use rand::rngs::StdRng;

use crate::storage::{Storage, StoredSettings};
use crate::strings::Language;

pub const FEEDBACK_DURATION: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    SessionSettings,
    Session,
    Result,
    Archive,
    Settings,
}

impl Screen {
    pub const ALL: [Screen; 6] = [
        Screen::Main,
        Screen::SessionSettings,
        Screen::Session,
        Screen::Result,
        Screen::Archive,
        Screen::Settings,
    ];

    /// Parses the desktop example's `--screen` values.
    pub fn from_arg(text: &str) -> Option<Self> {
        match text {
            "main" => Some(Self::Main),
            "session-settings" => Some(Self::SessionSettings),
            "session" => Some(Self::Session),
            "result" => Some(Self::Result),
            "archive" => Some(Self::Archive),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    EndSession,
    ClearArchive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    ConfigureFirst,
    Failure(String),
}

/// The task that was skipped after a wrong answer, shown for a short time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feedback {
    pub task: Task,
    pub remaining: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open(Screen),
    StartSession,
    AdjustDuration(i32),
    SetDifficulty(Difficulty),
    ToggleOperation(Operation),
    SetLanguage(Language),
    Digit(u8),
    Backspace,
    Submit,
    TogglePause,
    AskEndSession,
    ConfirmEndSession,
    AskClearArchive,
    ConfirmClearArchive,
    CancelDialog,
}

pub struct AppState {
    pub screen: Screen,
    pub language: Language,
    pub settings: Settings,
    /// Newest first.
    pub records: Vec<ArchiveRecord>,
    pub skipped_lines: usize,
    pub session: Option<Session>,
    /// Live counts for the session screen. The core keeps them private until the summary.
    pub correct: u32,
    pub incorrect: u32,
    pub input: String,
    pub feedback: Option<Feedback>,
    pub result: Option<SessionSummary>,
    pub dialog: Option<Dialog>,
    pub notice: Option<Notice>,
    resume_after_dialog: bool,
    storage: Storage,
    rng: StdRng,
}

impl AppState {
    pub fn new(storage: Storage, rng: StdRng) -> Self {
        let stored = storage.load_settings();
        let archive = storage.load_archive();
        let mut records = archive.records;
        records.sort_by_key(|record| std::cmp::Reverse(sort_key(&record.date)));
        Self {
            screen: Screen::Main,
            language: stored.language,
            settings: stored.settings,
            records,
            skipped_lines: archive.skipped,
            session: None,
            correct: 0,
            incorrect: 0,
            input: String::new(),
            feedback: None,
            result: None,
            dialog: None,
            notice: None,
            resume_after_dialog: false,
            storage,
            rng,
        }
    }

    /// Opens a screen directly, for the desktop preview. `Session` starts a
    /// real session and `Result` shows a canned summary that is not recorded.
    pub fn open_preview(&mut self, screen: Screen) {
        match screen {
            Screen::Session => self.start_session(),
            Screen::Result => {
                self.result = Some(sample_summary());
                self.go(Screen::Result);
            }
            other => self.go(other),
        }
    }

    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Open(screen) => self.go(screen),
            Action::StartSession => self.start_session(),
            Action::AdjustDuration(delta) => {
                let minutes = i64::from(self.settings.duration_minutes) + i64::from(delta);
                self.settings.duration_minutes = minutes.clamp(
                    i64::from(MIN_DURATION_MINUTES),
                    i64::from(MAX_DURATION_MINUTES),
                ) as u32;
                self.persist_settings();
            }
            Action::SetDifficulty(difficulty) => {
                self.settings.difficulty = difficulty;
                self.persist_settings();
            }
            Action::ToggleOperation(op) => {
                let on = !self.settings.operations.contains(op);
                self.settings.operations.set(op, on);
                self.persist_settings();
            }
            Action::SetLanguage(language) => {
                self.language = language;
                self.persist_settings();
            }
            Action::Digit(digit) => self.push_digit(digit),
            Action::Backspace => {
                if self.accepting_input() {
                    self.input.pop();
                }
            }
            Action::Submit => self.submit(),
            Action::TogglePause => self.toggle_pause(),
            Action::AskEndSession => self.ask_end_session(),
            Action::ConfirmEndSession => self.end_session_early(),
            Action::AskClearArchive => {
                if self.dialog.is_none() {
                    self.dialog = Some(Dialog::ClearArchive);
                }
            }
            Action::ConfirmClearArchive => self.clear_archive(),
            Action::CancelDialog => self.cancel_dialog(),
        }
    }

    /// Advances the session clock and the feedback timer. Finishes the session
    /// once its time is up. `now` is the timestamp recorded if it finishes.
    pub fn tick(&mut self, delta: Duration, now: Timestamp) {
        if let Some(feedback) = self.feedback.as_mut() {
            feedback.remaining = feedback.remaining.saturating_sub(delta);
            if feedback.remaining.is_zero() {
                self.feedback = None;
            }
        }
        if self.screen != Screen::Session || self.dialog.is_some() {
            return;
        }
        let Some(session) = self.session.as_mut() else {
            return;
        };
        session.tick(delta);
        if session.phase() == Phase::TimeUp {
            self.finish_session(now);
        }
    }

    fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.notice = None;
        self.dialog = None;
        if screen != Screen::Result {
            self.result = None;
        }
    }

    fn start_session(&mut self) {
        if self.settings.operations.is_empty() {
            self.notice = Some(Notice::ConfigureFirst);
            return;
        }
        match Session::start(&self.settings, &mut self.rng) {
            Ok(session) => {
                self.session = Some(session);
                self.correct = 0;
                self.incorrect = 0;
                self.input.clear();
                self.feedback = None;
                self.resume_after_dialog = false;
                self.go(Screen::Session);
            }
            Err(err) => self.notice = Some(Notice::Failure(err.to_string())),
        }
    }

    /// Input is accepted only while the session runs with no dialog open.
    fn accepting_input(&self) -> bool {
        self.dialog.is_none()
            && self
                .session
                .as_ref()
                .is_some_and(|session| session.phase() == Phase::Running)
    }

    fn push_digit(&mut self, digit: u8) {
        if digit < 10 && self.input.len() < MAX_ANSWER_DIGITS && self.accepting_input() {
            self.input.push(char::from(b'0' + digit));
        }
    }

    fn submit(&mut self) {
        if self.input.is_empty() || !self.accepting_input() {
            return;
        }
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let shown = session.current_task();
        match session.submit(&self.input, &mut self.rng) {
            Ok(Outcome::Incorrect) => {
                self.incorrect += 1;
                self.feedback = Some(Feedback {
                    task: shown,
                    remaining: FEEDBACK_DURATION,
                });
            }
            Ok(Outcome::Correct) => {
                self.correct += 1;
                self.feedback = None;
            }
            Ok(Outcome::Ignored) => {}
            Err(err) => self.notice = Some(Notice::Failure(err.to_string())),
        }
        self.input.clear();
    }

    fn toggle_pause(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        match session.phase() {
            Phase::Running => session.pause(),
            Phase::Paused => session.resume(),
            _ => {}
        }
    }

    fn ask_end_session(&mut self) {
        if self.dialog.is_some() {
            return;
        }
        let Some(session) = self.session.as_mut() else {
            return;
        };
        // Stop the clock while the dialog is open. A session the user paused
        // stays paused when the dialog is cancelled.
        if session.phase() == Phase::Running {
            session.pause();
            self.resume_after_dialog = true;
        }
        self.dialog = Some(Dialog::EndSession);
    }

    fn cancel_dialog(&mut self) {
        if self.dialog == Some(Dialog::EndSession)
            && self.resume_after_dialog
            && let Some(session) = self.session.as_mut()
        {
            session.resume();
        }
        self.resume_after_dialog = false;
        self.dialog = None;
    }

    /// Ending early discards the session. Nothing is recorded.
    fn end_session_early(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.discard().ok();
        }
        self.input.clear();
        self.feedback = None;
        self.resume_after_dialog = false;
        self.go(Screen::Main);
    }

    fn finish_session(&mut self, now: Timestamp) {
        let Some(mut session) = self.session.take() else {
            return;
        };
        self.input.clear();
        self.feedback = None;
        self.resume_after_dialog = false;
        let summary = match session.finish() {
            Ok(summary) => summary,
            Err(err) => {
                self.notice = Some(Notice::Failure(err.to_string()));
                self.go(Screen::Main);
                return;
            }
        };
        let record = ArchiveRecord::from_summary(&summary, now);
        if let Err(err) = self.storage.append_record(&record) {
            self.notice = Some(Notice::Failure(err.to_string()));
        }
        self.records.insert(0, record);
        self.result = Some(summary);
        self.go(Screen::Result);
    }

    fn clear_archive(&mut self) {
        match self.storage.clear_archive() {
            Ok(()) => {
                self.records.clear();
                self.skipped_lines = 0;
            }
            Err(err) => self.notice = Some(Notice::Failure(err.to_string())),
        }
        self.dialog = None;
    }

    fn persist_settings(&mut self) {
        let stored = StoredSettings {
            settings: self.settings.clone(),
            language: self.language,
        };
        if let Err(err) = self.storage.save_settings(&stored) {
            self.notice = Some(Notice::Failure(err.to_string()));
        }
    }
}

/// Grade for an archived session, using the same bands as a live result.
pub fn grade_for(tasks: u32, correct: u32) -> Grade {
    match (correct * 100).checked_div(tasks) {
        Some(percent) => Grade::from_percent(percent),
        None => Grade::NoTasks,
    }
}

/// Newest-first ordering key for archive records.
fn sort_key(t: &Timestamp) -> (u16, u8, u8, u8, u8, u8) {
    (t.year, t.month, t.day, t.hour, t.minute, t.second)
}

fn sample_summary() -> SessionSummary {
    SessionSummary {
        tasks: 20,
        correct: 15,
        incorrect: 5,
        percent_correct: 75,
        grade: Grade::Perfect,
        speed: 4,
        minutes: 5,
        difficulty: Difficulty::Medium,
        operations: [Operation::Add, Operation::Subtract].into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::ARCHIVE_FILE;
    use rand::SeedableRng;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let dir = std::env::temp_dir().join(format!(
                "mentalcalc-state-{name}-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    fn now() -> Timestamp {
        Timestamp::new(2014, 12, 11, 10, 30, 0).unwrap()
    }

    fn app(dir: &TempDir) -> AppState {
        AppState::new(Storage::new(&dir.0), StdRng::seed_from_u64(1))
    }

    fn type_number(state: &mut AppState, value: u32) {
        for ch in value.to_string().bytes() {
            state.apply(Action::Digit(ch - b'0'));
        }
    }

    fn current(state: &AppState) -> Task {
        state.session.as_ref().unwrap().current_task()
    }

    fn start(state: &mut AppState) {
        state.apply(Action::StartSession);
        assert_eq!(state.screen, Screen::Session);
    }

    fn seed_archive(dir: &TempDir, lines: &str) {
        fs::write(dir.0.join(ARCHIVE_FILE), lines).unwrap();
    }

    #[test]
    fn fresh_state_uses_defaults() {
        let dir = TempDir::new("fresh");
        let state = app(&dir);
        assert_eq!(state.screen, Screen::Main);
        assert_eq!(state.language, Language::Russian);
        assert_eq!(state.settings, Settings::default());
        assert!(state.records.is_empty());
        assert_eq!(state.skipped_lines, 0);
    }

    #[test]
    fn start_without_operations_stays_on_main_with_notice() {
        let dir = TempDir::new("no-ops");
        let mut state = app(&dir);
        state.apply(Action::ToggleOperation(Operation::Add));
        state.apply(Action::ToggleOperation(Operation::Subtract));
        state.apply(Action::StartSession);
        assert_eq!(state.screen, Screen::Main);
        assert_eq!(state.notice, Some(Notice::ConfigureFirst));
        assert!(state.session.is_none());
    }

    #[test]
    fn typing_is_capped_and_ignored_while_paused() {
        let dir = TempDir::new("keypad");
        let mut state = app(&dir);
        start(&mut state);
        for _ in 0..10 {
            state.apply(Action::Digit(9));
        }
        assert_eq!(state.input.len(), MAX_ANSWER_DIGITS);
        state.apply(Action::Backspace);
        assert_eq!(state.input.len(), MAX_ANSWER_DIGITS - 1);
        state.apply(Action::TogglePause);
        state.apply(Action::Digit(1));
        state.apply(Action::Backspace);
        assert_eq!(state.input.len(), MAX_ANSWER_DIGITS - 1);
    }

    #[test]
    fn correct_answer_clears_feedback_and_advances() {
        let dir = TempDir::new("correct");
        let mut state = app(&dir);
        start(&mut state);
        let task = current(&state);
        type_number(&mut state, task.answer);
        state.apply(Action::Submit);
        assert!(state.input.is_empty());
        assert!(state.feedback.is_none());
        assert_eq!(state.session.as_ref().unwrap().tasks_answered(), 1);
    }

    #[test]
    fn wrong_answer_shows_the_skipped_task_briefly() {
        let dir = TempDir::new("wrong");
        let mut state = app(&dir);
        start(&mut state);
        let task = current(&state);
        type_number(&mut state, task.answer + 1);
        state.apply(Action::Submit);
        let feedback = state.feedback.expect("feedback after a wrong answer");
        assert_eq!(feedback.task, task);
        assert_eq!(state.session.as_ref().unwrap().tasks_answered(), 1);
        assert_eq!((state.correct, state.incorrect), (0, 1));
        state.tick(Duration::from_secs(1), now());
        assert!(state.feedback.is_some());
        state.tick(Duration::from_secs(1), now());
        assert!(state.feedback.is_none());
    }

    #[test]
    fn timer_end_saves_one_record_and_shows_result() {
        let dir = TempDir::new("timer");
        let mut state = app(&dir);
        state.apply(Action::AdjustDuration(-10));
        assert_eq!(state.settings.duration_minutes, MIN_DURATION_MINUTES);
        start(&mut state);
        let task = current(&state);
        type_number(&mut state, task.answer);
        state.apply(Action::Submit);

        state.tick(Duration::from_secs(60), now());
        assert_eq!(state.screen, Screen::Result);
        assert!(state.session.is_none());
        let summary = state.result.clone().expect("result after time is up");
        assert_eq!((summary.tasks, summary.correct), (1, 1));
        assert_eq!(state.records.len(), 1);

        state.tick(Duration::from_secs(60), now());
        assert_eq!(state.records.len(), 1);
        let saved = Storage::new(&dir.0).load_archive();
        assert_eq!(saved.records.len(), 1);
        assert_eq!(saved.records[0].date, now());
    }

    #[test]
    fn ending_early_pauses_the_clock_and_cancel_resumes_it() {
        let dir = TempDir::new("pause-dialog");
        let mut state = app(&dir);
        start(&mut state);
        state.tick(Duration::from_secs(10), now());
        state.apply(Action::AskEndSession);
        assert_eq!(state.dialog, Some(Dialog::EndSession));
        state.tick(Duration::from_secs(5), now());
        assert_eq!(
            state.session.as_ref().unwrap().elapsed(),
            Duration::from_secs(10)
        );
        state.apply(Action::CancelDialog);
        assert_eq!(state.dialog, None);
        assert_eq!(state.session.as_ref().unwrap().phase(), Phase::Running);
        state.tick(Duration::from_secs(5), now());
        assert_eq!(
            state.session.as_ref().unwrap().elapsed(),
            Duration::from_secs(15)
        );
    }

    #[test]
    fn user_pause_survives_cancelling_the_end_dialog() {
        let dir = TempDir::new("user-pause");
        let mut state = app(&dir);
        start(&mut state);
        state.apply(Action::TogglePause);
        state.apply(Action::AskEndSession);
        state.apply(Action::CancelDialog);
        assert_eq!(state.session.as_ref().unwrap().phase(), Phase::Paused);
    }

    #[test]
    fn ending_early_discards_without_saving() {
        let dir = TempDir::new("discard");
        let mut state = app(&dir);
        start(&mut state);
        let task = current(&state);
        type_number(&mut state, task.answer);
        state.apply(Action::Submit);
        state.apply(Action::AskEndSession);
        state.apply(Action::ConfirmEndSession);
        assert_eq!(state.screen, Screen::Main);
        assert!(state.session.is_none());
        assert!(state.records.is_empty());
        assert!(Storage::new(&dir.0).load_archive().records.is_empty());
    }

    #[test]
    fn settings_persist_across_restarts() {
        let dir = TempDir::new("settings");
        {
            let mut state = app(&dir);
            state.apply(Action::AdjustDuration(100));
            state.apply(Action::SetDifficulty(Difficulty::Expert));
            state.apply(Action::ToggleOperation(Operation::Multiply));
            state.apply(Action::SetLanguage(Language::English));
            assert_eq!(state.settings.duration_minutes, MAX_DURATION_MINUTES);
        }
        let state = app(&dir);
        assert_eq!(state.settings.duration_minutes, MAX_DURATION_MINUTES);
        assert_eq!(state.settings.difficulty, Difficulty::Expert);
        assert!(state.settings.operations.contains(Operation::Multiply));
        assert_eq!(state.language, Language::English);
    }

    #[test]
    fn duration_never_leaves_its_range() {
        let dir = TempDir::new("duration");
        let mut state = app(&dir);
        for _ in 0..30 {
            state.apply(Action::AdjustDuration(-1));
        }
        assert_eq!(state.settings.duration_minutes, MIN_DURATION_MINUTES);
        for _ in 0..30 {
            state.apply(Action::AdjustDuration(1));
        }
        assert_eq!(state.settings.duration_minutes, MAX_DURATION_MINUTES);
    }

    #[test]
    fn archive_is_newest_first_and_reports_skipped_lines() {
        let dir = TempDir::new("archive-order");
        seed_archive(
            &dir,
            "2014/12/10_09:00:00 10 5 5 5 2 1 0 0 0\n\
             not a record\n\
             2014/12/11_10:30:00 20 15 5 5 2 1 0 0 0\n\
             2014/12/10_18:00:00 10 9 1 5 2 1 0 0 0\n",
        );
        let state = app(&dir);
        let dates: Vec<u8> = state.records.iter().map(|r| r.date.hour).collect();
        assert_eq!(dates, vec![10, 18, 9]);
        assert_eq!(state.skipped_lines, 1);
    }

    #[test]
    fn clearing_the_archive_needs_confirmation() {
        let dir = TempDir::new("clear");
        seed_archive(&dir, "2014/12/11_10:30:00 20 15 5 5 2 1 0 0 0\n");
        let mut state = app(&dir);
        assert_eq!(state.records.len(), 1);
        state.apply(Action::AskClearArchive);
        assert_eq!(state.dialog, Some(Dialog::ClearArchive));
        state.apply(Action::CancelDialog);
        assert_eq!(state.records.len(), 1);
        state.apply(Action::AskClearArchive);
        state.apply(Action::ConfirmClearArchive);
        assert!(state.records.is_empty());
        assert!(!dir.0.join(ARCHIVE_FILE).exists());
    }

    #[test]
    fn preview_result_uses_canned_summary_without_recording() {
        let dir = TempDir::new("preview");
        let mut state = app(&dir);
        state.open_preview(Screen::Result);
        assert_eq!(state.screen, Screen::Result);
        assert_eq!(state.result, Some(sample_summary()));
        assert!(state.records.is_empty());
        assert!(!dir.0.join(ARCHIVE_FILE).exists());
    }

    #[test]
    fn grade_bands_for_archived_sessions() {
        assert_eq!(grade_for(0, 0), Grade::NoTasks);
        assert_eq!(grade_for(20, 14), Grade::Perfect);
        assert_eq!(grade_for(20, 7), Grade::Bad);
        assert_eq!(grade_for(20, 0), Grade::VeryBad);
    }

    #[test]
    fn screen_arguments_parse() {
        for screen in Screen::ALL {
            let name = match screen {
                Screen::Main => "main",
                Screen::SessionSettings => "session-settings",
                Screen::Session => "session",
                Screen::Result => "result",
                Screen::Archive => "archive",
                Screen::Settings => "settings",
            };
            assert_eq!(Screen::from_arg(name), Some(screen));
        }
        assert_eq!(Screen::from_arg("nope"), None);
    }
}
