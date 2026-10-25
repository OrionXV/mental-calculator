//! One session per line, fields separated by single spaces:
//! `yyyy/MM/dd_HH:mm:ss tasks correct incorrect minutes difficulty add subtract multiply divide`
//! Operation flags are 1 or 0, in that order.

use std::fmt;
use std::str::FromStr;

use crate::difficulty::Difficulty;
use crate::operation::{Operation, OperationSet};
use crate::session::SessionSummary;
use crate::settings::{MAX_DURATION_MINUTES, MIN_DURATION_MINUTES};

const FIELD_COUNT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Timestamp {
    pub fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<Self> {
        let valid = year <= 9999
            && (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day)
            && hour < 24
            && minute < 60
            && second < 60;
        valid.then_some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}/{:02}/{:02}_{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn is_leap(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveRecord {
    pub date: Timestamp,
    pub tasks: u32,
    pub correct: u32,
    pub incorrect: u32,
    pub minutes: u32,
    pub difficulty: Difficulty,
    pub operations: OperationSet,
}

impl ArchiveRecord {
    pub fn from_summary(summary: &SessionSummary, date: Timestamp) -> Self {
        Self {
            date,
            tasks: summary.tasks,
            correct: summary.correct,
            incorrect: summary.incorrect,
            minutes: summary.minutes,
            difficulty: summary.difficulty,
            operations: summary.operations,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveError {
    FieldCount(usize),
    InvalidDate(String),
    InvalidNumber(&'static str),
    OutOfRange(&'static str),
    InvalidFlag(&'static str),
    CountMismatch,
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldCount(found) => write!(f, "expected {FIELD_COUNT} fields, found {found}"),
            Self::InvalidDate(text) => write!(f, "invalid date: {text:?}"),
            Self::InvalidNumber(field) => write!(f, "invalid number in {field}"),
            Self::OutOfRange(field) => write!(f, "value out of range in {field}"),
            Self::InvalidFlag(field) => write!(f, "invalid flag in {field}"),
            Self::CountMismatch => write!(f, "correct plus incorrect does not equal tasks"),
        }
    }
}

impl std::error::Error for ArchiveError {}

pub fn format_line(record: &ArchiveRecord) -> String {
    let bit = |op| u8::from(record.operations.contains(op));
    format!(
        "{} {} {} {} {} {} {} {} {} {}",
        record.date,
        record.tasks,
        record.correct,
        record.incorrect,
        record.minutes,
        record.difficulty.code(),
        bit(Operation::Add),
        bit(Operation::Subtract),
        bit(Operation::Multiply),
        bit(Operation::Divide),
    )
}

pub fn parse_line(line: &str) -> Result<ArchiveRecord, ArchiveError> {
    let fields: Vec<&str> = line.split(' ').collect();
    let [
        date,
        tasks,
        correct,
        incorrect,
        minutes,
        difficulty,
        add,
        subtract,
        multiply,
        divide,
    ] = fields.as_slice()
    else {
        return Err(ArchiveError::FieldCount(fields.len()));
    };

    let date = parse_date(date)?;
    let tasks = number(tasks, "tasks")?;
    let correct = number(correct, "correct")?;
    let incorrect = number(incorrect, "incorrect")?;
    let minutes = number(minutes, "minutes")?;
    if !(MIN_DURATION_MINUTES..=MAX_DURATION_MINUTES).contains(&minutes) {
        return Err(ArchiveError::OutOfRange("minutes"));
    }
    let difficulty = number(difficulty, "difficulty")
        .ok()
        .and_then(|code| u8::try_from(code).ok())
        .and_then(Difficulty::from_code)
        .ok_or(ArchiveError::OutOfRange("difficulty"))?;

    let mut operations = OperationSet::empty();
    operations.set(Operation::Add, flag(add, "add")?);
    operations.set(Operation::Subtract, flag(subtract, "subtract")?);
    operations.set(Operation::Multiply, flag(multiply, "multiply")?);
    operations.set(Operation::Divide, flag(divide, "divide")?);

    if u64::from(correct) + u64::from(incorrect) != u64::from(tasks) {
        return Err(ArchiveError::CountMismatch);
    }

    Ok(ArchiveRecord {
        date,
        tasks,
        correct,
        incorrect,
        minutes,
        difficulty,
        operations,
    })
}

fn parse_date(text: &str) -> Result<Timestamp, ArchiveError> {
    let invalid = || ArchiveError::InvalidDate(text.to_owned());
    let (date, time) = text.split_once('_').ok_or_else(invalid)?;
    let date: Vec<&str> = date.split('/').collect();
    let time: Vec<&str> = time.split(':').collect();
    let (&[year, month, day], &[hour, minute, second]) = (date.as_slice(), time.as_slice()) else {
        return Err(invalid());
    };
    Timestamp::new(
        fixed_digits(year, 4).ok_or_else(invalid)?,
        fixed_digits(month, 2).ok_or_else(invalid)?,
        fixed_digits(day, 2).ok_or_else(invalid)?,
        fixed_digits(hour, 2).ok_or_else(invalid)?,
        fixed_digits(minute, 2).ok_or_else(invalid)?,
        fixed_digits(second, 2).ok_or_else(invalid)?,
    )
    .ok_or_else(invalid)
}

fn fixed_digits<T: FromStr>(text: &str, width: usize) -> Option<T> {
    if text.len() == width && text.bytes().all(|b| b.is_ascii_digit()) {
        text.parse().ok()
    } else {
        None
    }
}

fn number(text: &str, field: &'static str) -> Result<u32, ArchiveError> {
    let digits = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    digits
        .then(|| text.parse::<u32>().ok())
        .flatten()
        .ok_or(ArchiveError::InvalidNumber(field))
}

fn flag(text: &str, field: &'static str) -> Result<bool, ArchiveError> {
    match text {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(ArchiveError::InvalidFlag(field)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> ArchiveRecord {
        ArchiveRecord {
            date: Timestamp::new(2014, 12, 11, 10, 30, 5).unwrap(),
            tasks: 20,
            correct: 15,
            incorrect: 5,
            minutes: 5,
            difficulty: Difficulty::Hard,
            operations: [Operation::Add, Operation::Divide].into_iter().collect(),
        }
    }

    #[test]
    fn format_matches_original_field_order() {
        assert_eq!(
            format_line(&record()),
            "2014/12/11_10:30:05 20 15 5 5 3 1 0 0 1"
        );
    }

    #[test]
    fn format_then_parse_round_trips() {
        let mut other = record();
        other.tasks = 0;
        other.correct = 0;
        other.incorrect = 0;
        other.operations = OperationSet::empty();
        for original in [record(), other] {
            let line = format_line(&original);
            assert_eq!(parse_line(&line), Ok(original));
        }
    }

    #[test]
    fn parses_a_line_written_by_the_original_app() {
        let parsed = parse_line("2014/12/11_10:30:05 20 15 5 5 2 1 1 0 0").unwrap();
        assert_eq!(parsed.tasks, 20);
        assert_eq!(parsed.difficulty, Difficulty::Medium);
        assert!(parsed.operations.contains(Operation::Add));
        assert!(parsed.operations.contains(Operation::Subtract));
        assert!(!parsed.operations.contains(Operation::Multiply));
        assert!(!parsed.operations.contains(Operation::Divide));
    }

    #[test]
    fn rejects_malformed_lines_without_panicking() {
        let bad = [
            "",
            " ",
            "2014/12/11_10:30:05 20 15 5 5 2 1 1 0",
            "2014/12/11_10:30:05 20 15 5 5 2 1 1 0 0 extra",
            "2014-12-11_10:30:05 20 15 5 5 2 1 1 0 0",
            "2014/13/11_10:30:05 20 15 5 5 2 1 1 0 0",
            "2015/02/29_10:30:05 20 15 5 5 2 1 1 0 0",
            "2014/04/31_10:30:00 20 15 5 5 2 1 1 0 0",
            "2014/12/11_24:00:00 20 15 5 5 2 1 1 0 0",
            "2014/12/11_10:30 20 15 5 5 2 1 1 0 0",
            "2014/12/11_10:30:05 x 15 5 5 2 1 1 0 0",
            "2014/12/11_10:30:05 -20 15 5 5 2 1 1 0 0",
            "2014/12/11_10:30:05 +20 15 5 5 2 1 1 0 0",
            "2014/12/11_10:30:05 20 15 5 0 2 1 1 0 0",
            "2014/12/11_10:30:05 20 15 5 21 2 1 1 0 0",
            "2014/12/11_10:30:05 20 15 5 5 0 1 1 0 0",
            "2014/12/11_10:30:05 20 15 5 5 5 1 1 0 0",
            "2014/12/11_10:30:05 20 15 5 5 2 2 1 0 0",
            "2014/12/11_10:30:05 20 15 5 5 2 1 1 0 true",
            "2014/12/11_10:30:05 20 16 5 5 2 1 1 0 0",
            "2014/12/11_10:30:05 4294967295 4294967295 1 5 2 1 1 0 0",
            "2014/12/11_10:30:05 20 15 5 5 2 1 1 0 0\u{0}",
            "\u{1F600}\u{1F600}",
        ];
        for line in bad {
            assert!(parse_line(line).is_err(), "accepted: {line:?}");
        }
    }

    #[test]
    fn timestamp_validates_calendar_fields() {
        assert!(Timestamp::new(2016, 2, 29, 0, 0, 0).is_some());
        assert!(Timestamp::new(2000, 2, 29, 0, 0, 0).is_some());
        assert!(Timestamp::new(1900, 2, 29, 0, 0, 0).is_none());
        assert!(Timestamp::new(2014, 12, 31, 23, 59, 59).is_some());
        assert!(Timestamp::new(2014, 0, 1, 0, 0, 0).is_none());
    }
}
