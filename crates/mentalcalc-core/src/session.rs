use std::fmt;
use std::time::Duration;

use rand::Rng;

use crate::difficulty::Difficulty;
use crate::operation::OperationSet;
use crate::settings::{MAX_DURATION_MINUTES, MIN_DURATION_MINUTES, Settings};
use crate::task::{Task, TaskGenerator};

/// The keypad accepts at most 7 digits in the original.
pub const MAX_ANSWER_DIGITS: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Running,
    Paused,
    /// Planned time has run out. Only `finish` is accepted now.
    TimeUp,
    /// The summary has been produced; the session can no longer be finished.
    Completed,
    /// Ended early. Nothing is recorded.
    Discarded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Correct,
    Incorrect,
    Ignored,
}

/// Bands: below 20, 20 to 39, 40 to 69, 70 and up. `NoTasks` mirrors the
/// original's "did not even try" result for a session with zero answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    NoTasks,
    VeryBad,
    Bad,
    Normal,
    Perfect,
}

impl Grade {
    pub fn from_percent(percent: u32) -> Self {
        match percent {
            0..20 => Self::VeryBad,
            20..40 => Self::Bad,
            40..70 => Self::Normal,
            _ => Self::Perfect,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub tasks: u32,
    pub correct: u32,
    pub incorrect: u32,
    pub percent_correct: u32,
    pub grade: Grade,
    /// Tasks answered per planned minute, rounded down.
    pub speed: u32,
    pub minutes: u32,
    pub difficulty: Difficulty,
    pub operations: OperationSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    InvalidDuration(u32),
    NoOperations,
    InvalidAnswer,
    WrongPhase(Phase),
    AlreadyFinished,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDuration(minutes) => write!(f, "invalid session duration: {minutes}"),
            Self::NoOperations => write!(f, "no operation is enabled"),
            Self::InvalidAnswer => write!(f, "answer must be up to {MAX_ANSWER_DIGITS} digits"),
            Self::WrongPhase(phase) => write!(f, "action not allowed in phase {phase:?}"),
            Self::AlreadyFinished => write!(f, "session was already finished"),
        }
    }
}

impl std::error::Error for SessionError {}

/// Timed session. The caller drives time through `tick` and owns the clock.
#[derive(Debug)]
pub struct Session {
    minutes: u32,
    difficulty: Difficulty,
    operations: OperationSet,
    duration: Duration,
    elapsed: Duration,
    phase: Phase,
    generator: TaskGenerator,
    current: Task,
    correct: u32,
    incorrect: u32,
}

impl Session {
    pub fn start<R: Rng + ?Sized>(settings: &Settings, rng: &mut R) -> Result<Self, SessionError> {
        let minutes = settings.duration_minutes;
        if !(MIN_DURATION_MINUTES..=MAX_DURATION_MINUTES).contains(&minutes) {
            return Err(SessionError::InvalidDuration(minutes));
        }
        let mut generator = TaskGenerator::new(settings.difficulty, settings.operations)
            .ok_or(SessionError::NoOperations)?;
        let current = generator.next_task(rng);
        Ok(Self {
            minutes,
            difficulty: settings.difficulty,
            operations: settings.operations,
            duration: Duration::from_secs(u64::from(minutes) * 60),
            elapsed: Duration::ZERO,
            phase: Phase::Running,
            generator,
            current,
            correct: 0,
            incorrect: 0,
        })
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn remaining(&self) -> Duration {
        self.duration - self.elapsed
    }

    pub fn current_task(&self) -> Task {
        self.current
    }

    pub fn tasks_answered(&self) -> u32 {
        self.correct + self.incorrect
    }

    /// Advances the clock by `delta`. Ignored unless the session is running.
    pub fn tick(&mut self, delta: Duration) {
        if self.phase != Phase::Running {
            return;
        }
        self.elapsed = self.elapsed.saturating_add(delta).min(self.duration);
        if self.elapsed == self.duration {
            self.phase = Phase::TimeUp;
        }
    }

    pub fn pause(&mut self) {
        if self.phase == Phase::Running {
            self.phase = Phase::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = Phase::Running;
        }
    }

    /// An empty input is ignored. A wrong answer is counted and the task is skipped.
    pub fn submit<R: Rng + ?Sized>(
        &mut self,
        input: &str,
        rng: &mut R,
    ) -> Result<Outcome, SessionError> {
        if self.phase != Phase::Running {
            return Err(SessionError::WrongPhase(self.phase));
        }
        if input.is_empty() {
            return Ok(Outcome::Ignored);
        }
        let answer = parse_answer(input)?;
        let outcome = if answer == self.current.answer {
            self.correct += 1;
            Outcome::Correct
        } else {
            self.incorrect += 1;
            Outcome::Incorrect
        };
        self.current = self.generator.next_task(rng);
        Ok(outcome)
    }

    /// Ends the session early. No summary is produced and nothing is recorded.
    pub fn discard(&mut self) -> Result<(), SessionError> {
        match self.phase {
            Phase::Running | Phase::Paused => {
                self.phase = Phase::Discarded;
                Ok(())
            }
            phase => Err(SessionError::WrongPhase(phase)),
        }
    }

    /// Returns the summary once, after time is up. Later calls fail.
    pub fn finish(&mut self) -> Result<SessionSummary, SessionError> {
        match self.phase {
            Phase::TimeUp => {
                self.phase = Phase::Completed;
                Ok(self.summary())
            }
            Phase::Completed => Err(SessionError::AlreadyFinished),
            phase => Err(SessionError::WrongPhase(phase)),
        }
    }

    fn summary(&self) -> SessionSummary {
        let tasks = self.tasks_answered();
        let percent_correct = (self.correct * 100).checked_div(tasks).unwrap_or(0);
        let grade = if tasks == 0 {
            Grade::NoTasks
        } else {
            Grade::from_percent(percent_correct)
        };
        SessionSummary {
            tasks,
            correct: self.correct,
            incorrect: self.incorrect,
            percent_correct,
            grade,
            speed: tasks / self.minutes,
            minutes: self.minutes,
            difficulty: self.difficulty,
            operations: self.operations,
        }
    }
}

fn parse_answer(input: &str) -> Result<u32, SessionError> {
    if input.len() > MAX_ANSWER_DIGITS || !input.bytes().all(|b| b.is_ascii_digit()) {
        return Err(SessionError::InvalidAnswer);
    }
    input.parse().map_err(|_| SessionError::InvalidAnswer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::{ArchiveRecord, Timestamp, format_line};
    use crate::difficulty::DEFAULT_DIFFICULTY;
    use crate::operation::Operation;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(1)
    }

    fn settings(minutes: u32, ops: &[Operation]) -> Settings {
        Settings {
            duration_minutes: minutes,
            difficulty: Difficulty::Easy,
            operations: ops.iter().copied().collect(),
        }
    }

    fn started(minutes: u32, ops: &[Operation], rng: &mut StdRng) -> Session {
        Session::start(&settings(minutes, ops), rng).unwrap()
    }

    fn answer_right(session: &mut Session, rng: &mut StdRng) -> Outcome {
        let answer = session.current_task().answer;
        session.submit(&answer.to_string(), rng).unwrap()
    }

    fn answer_wrong(session: &mut Session, rng: &mut StdRng) -> Outcome {
        let answer = session.current_task().answer;
        session.submit(&(answer + 1).to_string(), rng).unwrap()
    }

    #[test]
    fn timer_counts_down_to_time_up() {
        let mut rng = rng();
        let mut s = started(1, &[Operation::Add], &mut rng);
        assert_eq!(s.remaining(), Duration::from_secs(60));
        s.tick(Duration::from_secs(59));
        assert_eq!(s.phase(), Phase::Running);
        assert_eq!(s.remaining(), Duration::from_secs(1));
        s.tick(Duration::from_secs(1));
        assert_eq!(s.phase(), Phase::TimeUp);
        assert_eq!(s.remaining(), Duration::ZERO);
        s.tick(Duration::from_secs(5));
        assert_eq!(s.elapsed(), Duration::from_secs(60));
    }

    #[test]
    fn pause_stops_the_clock_and_resume_restarts_it() {
        let mut rng = rng();
        let mut s = started(2, &[Operation::Add], &mut rng);
        s.tick(Duration::from_secs(10));
        s.pause();
        assert_eq!(s.phase(), Phase::Paused);
        s.tick(Duration::from_secs(30));
        assert_eq!(s.elapsed(), Duration::from_secs(10));
        assert_eq!(
            s.submit("1", &mut rng),
            Err(SessionError::WrongPhase(Phase::Paused))
        );
        s.resume();
        assert_eq!(s.phase(), Phase::Running);
        s.tick(Duration::from_secs(5));
        assert_eq!(s.elapsed(), Duration::from_secs(15));
    }

    #[test]
    fn empty_input_is_ignored() {
        let mut rng = rng();
        let mut s = started(1, &[Operation::Add], &mut rng);
        let before = s.current_task();
        assert_eq!(s.submit("", &mut rng), Ok(Outcome::Ignored));
        assert_eq!(s.tasks_answered(), 0);
        assert_eq!(s.current_task(), before);
    }

    #[test]
    fn correct_answer_is_counted_and_next_task_is_drawn() {
        let mut rng = rng();
        let mut s = started(1, &[Operation::Add], &mut rng);
        let before = s.current_task();
        assert_eq!(answer_right(&mut s, &mut rng), Outcome::Correct);
        assert_eq!(s.tasks_answered(), 1);
        let after = s.current_task();
        assert_ne!((before.first, before.second), (after.first, after.second));
    }

    #[test]
    fn wrong_answer_is_counted_and_skipped() {
        let mut rng = rng();
        let mut s = started(1, &[Operation::Add], &mut rng);
        let before = s.current_task();
        assert_eq!(answer_wrong(&mut s, &mut rng), Outcome::Incorrect);
        assert_ne!(before, s.current_task());
        s.tick(Duration::from_secs(60));
        let summary = s.finish().unwrap();
        assert_eq!((summary.correct, summary.incorrect), (0, 1));
    }

    #[test]
    fn invalid_input_is_rejected_without_counting() {
        let mut rng = rng();
        let mut s = started(1, &[Operation::Add], &mut rng);
        assert_eq!(s.submit("12a", &mut rng), Err(SessionError::InvalidAnswer));
        assert_eq!(s.submit("-3", &mut rng), Err(SessionError::InvalidAnswer));
        assert_eq!(
            s.submit("12345678", &mut rng),
            Err(SessionError::InvalidAnswer)
        );
        assert_eq!(s.tasks_answered(), 0);
    }

    #[test]
    fn expired_session_rejects_answers() {
        let mut rng = rng();
        let mut s = started(1, &[Operation::Add], &mut rng);
        s.tick(Duration::from_secs(60));
        assert_eq!(
            s.submit("1", &mut rng),
            Err(SessionError::WrongPhase(Phase::TimeUp))
        );
    }

    #[test]
    fn early_end_discards_the_session() {
        let mut rng = rng();
        let mut s = started(1, &[Operation::Add], &mut rng);
        answer_right(&mut s, &mut rng);
        s.discard().unwrap();
        assert_eq!(s.phase(), Phase::Discarded);
        assert_eq!(s.finish(), Err(SessionError::WrongPhase(Phase::Discarded)));
        assert_eq!(s.discard(), Err(SessionError::WrongPhase(Phase::Discarded)));
    }

    #[test]
    fn finish_before_time_up_is_rejected() {
        let mut rng = rng();
        let mut s = started(1, &[Operation::Add], &mut rng);
        assert_eq!(s.finish(), Err(SessionError::WrongPhase(Phase::Running)));
    }

    #[test]
    fn finish_returns_summary_only_once() {
        let mut rng = rng();
        let mut s = started(1, &[Operation::Add], &mut rng);
        answer_right(&mut s, &mut rng);
        s.tick(Duration::from_secs(60));
        assert!(s.finish().is_ok());
        assert_eq!(s.finish(), Err(SessionError::AlreadyFinished));
        assert_eq!(s.phase(), Phase::Completed);
        assert_eq!(
            s.submit("1", &mut rng),
            Err(SessionError::WrongPhase(Phase::Completed))
        );
    }

    #[test]
    fn summary_rounds_percent_down_and_speed_down() {
        let mut rng = rng();
        let mut s = started(5, &[Operation::Add], &mut rng);
        answer_right(&mut s, &mut rng);
        answer_right(&mut s, &mut rng);
        answer_wrong(&mut s, &mut rng);
        s.tick(Duration::from_secs(300));
        let summary = s.finish().unwrap();
        assert_eq!(summary.tasks, 3);
        assert_eq!(summary.correct, 2);
        assert_eq!(summary.incorrect, 1);
        assert_eq!(summary.percent_correct, 66);
        assert_eq!(summary.grade, Grade::Normal);
        assert_eq!(summary.speed, 0);
        assert_eq!(summary.minutes, 5);
    }

    #[test]
    fn summary_speed_is_tasks_per_planned_minute() {
        let mut rng = rng();
        let mut s = started(5, &[Operation::Add], &mut rng);
        for i in 0..7 {
            if i < 5 {
                answer_right(&mut s, &mut rng);
            } else {
                answer_wrong(&mut s, &mut rng);
            }
        }
        s.tick(Duration::from_secs(300));
        let summary = s.finish().unwrap();
        assert_eq!(summary.percent_correct, 71);
        assert_eq!(summary.grade, Grade::Perfect);
        assert_eq!(summary.speed, 1);
    }

    #[test]
    fn session_with_no_answers_has_no_tasks_grade() {
        let mut rng = rng();
        let mut s = started(3, &[Operation::Add], &mut rng);
        s.tick(Duration::from_secs(180));
        let summary = s.finish().unwrap();
        assert_eq!(summary.tasks, 0);
        assert_eq!(summary.percent_correct, 0);
        assert_eq!(summary.speed, 0);
        assert_eq!(summary.grade, Grade::NoTasks);
    }

    #[test]
    fn grade_bands_match_original_edges() {
        assert_eq!(Grade::from_percent(0), Grade::VeryBad);
        assert_eq!(Grade::from_percent(19), Grade::VeryBad);
        assert_eq!(Grade::from_percent(20), Grade::Bad);
        assert_eq!(Grade::from_percent(39), Grade::Bad);
        assert_eq!(Grade::from_percent(40), Grade::Normal);
        assert_eq!(Grade::from_percent(69), Grade::Normal);
        assert_eq!(Grade::from_percent(70), Grade::Perfect);
        assert_eq!(Grade::from_percent(100), Grade::Perfect);
    }

    #[test]
    fn duration_must_be_between_one_and_twenty_minutes() {
        let mut rng = rng();
        assert_eq!(
            Session::start(&settings(0, &[Operation::Add]), &mut rng).err(),
            Some(SessionError::InvalidDuration(0))
        );
        assert_eq!(
            Session::start(&settings(21, &[Operation::Add]), &mut rng).err(),
            Some(SessionError::InvalidDuration(21))
        );
        assert!(Session::start(&settings(1, &[Operation::Add]), &mut rng).is_ok());
        assert!(Session::start(&settings(20, &[Operation::Add]), &mut rng).is_ok());
    }

    #[test]
    fn session_requires_an_enabled_operation() {
        let mut rng = rng();
        assert_eq!(
            Session::start(&settings(1, &[]), &mut rng).err(),
            Some(SessionError::NoOperations)
        );
    }

    /// Regression: the original saved difficulty 1 for generation but 3 in the archive.
    #[test]
    fn default_difficulty_is_consistent_for_session_and_archive() {
        let mut rng = rng();
        let mut s = Session::start(&Settings::default(), &mut rng).unwrap();
        s.tick(Duration::from_secs(5 * 60));
        let summary = s.finish().unwrap();
        assert_eq!(summary.difficulty, DEFAULT_DIFFICULTY);
        let when = Timestamp::new(2014, 12, 11, 10, 30, 0).unwrap();
        let line = format_line(&ArchiveRecord::from_summary(&summary, when));
        assert_eq!(line.split(' ').nth(5), Some("2"));
    }
}
