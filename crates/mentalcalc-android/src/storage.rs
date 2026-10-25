//! File storage in a data directory: `settings.txt` and `sessions_results.mcd`,
//! using the core's formats. The language is stored as an extra `language=` line
//! in `settings.txt`, which the core parser ignores.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use mentalcalc_core::archive::{Timestamp, format_line, parse_line};
use mentalcalc_core::{ArchiveRecord, Settings};

use crate::strings::Language;

pub const SETTINGS_FILE: &str = "settings.txt";
pub const ARCHIVE_FILE: &str = "sessions_results.mcd";
const LANGUAGE_KEY: &str = "language";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StoredSettings {
    pub settings: Settings,
    pub language: Language,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ArchiveLoad {
    /// Records in file order (oldest first).
    pub records: Vec<ArchiveRecord>,
    /// Non-blank lines that could not be parsed and were skipped.
    pub skipped: usize,
}

#[derive(Debug, Clone)]
pub struct Storage {
    dir: PathBuf,
}

impl Storage {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Missing or unreadable settings fall back to defaults.
    pub fn load_settings(&self) -> StoredSettings {
        fs::read_to_string(self.dir.join(SETTINGS_FILE))
            .map(|text| parse_settings(&text))
            .unwrap_or_default()
    }

    pub fn save_settings(&self, stored: &StoredSettings) -> io::Result<()> {
        let text = format!(
            "{}{LANGUAGE_KEY}={}\n",
            stored.settings.format(),
            stored.language.code()
        );
        self.write_atomic(SETTINGS_FILE, text.as_bytes())
    }

    /// A missing archive is empty. Malformed lines are skipped and counted.
    pub fn load_archive(&self) -> ArchiveLoad {
        match fs::read(self.dir.join(ARCHIVE_FILE)) {
            Ok(bytes) => parse_archive(&bytes),
            Err(_) => ArchiveLoad::default(),
        }
    }

    pub fn append_record(&self, record: &ArchiveRecord) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(ARCHIVE_FILE);
        let existing = fs::read(&path).unwrap_or_default();
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        // A previous write may have ended without a newline; keep records on their own lines.
        let prefix = if existing.last().is_some_and(|&byte| byte != b'\n') {
            "\n"
        } else {
            ""
        };
        writeln!(file, "{prefix}{}", format_line(record))
    }

    pub fn clear_archive(&self) -> io::Result<()> {
        match fs::remove_file(self.dir.join(ARCHIVE_FILE)) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }

    fn write_atomic(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let target = self.dir.join(name);
        let temp = self.dir.join(format!("{name}.tmp"));
        fs::write(&temp, bytes)?;
        fs::rename(temp, target)
    }
}

pub fn parse_settings(text: &str) -> StoredSettings {
    let language = text
        .lines()
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| key.trim() == LANGUAGE_KEY)
        .and_then(|(_, value)| Language::from_code(value.trim()))
        .unwrap_or_default();
    StoredSettings {
        settings: Settings::parse(text),
        language,
    }
}

/// Parses raw bytes so that invalid UTF-8 counts as a malformed line, not a failed load.
pub fn parse_archive(bytes: &[u8]) -> ArchiveLoad {
    let text = String::from_utf8_lossy(bytes);
    let mut load = ArchiveLoad::default();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(line) {
            Ok(record) => load.records.push(record),
            Err(_) => load.skipped += 1,
        }
    }
    load
}

/// Current time as a UTC timestamp. The core has no time zone data, so the
/// archive stores UTC on every platform.
pub fn now() -> Timestamp {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0);
    timestamp_from_unix(secs)
}

pub fn timestamp_from_unix(secs: i64) -> Timestamp {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    Timestamp {
        year: u16::try_from(year.clamp(0, 9999)).unwrap_or(9999),
        month: month as u8,
        day: day as u8,
        hour: (rem / 3600) as u8,
        minute: (rem % 3600 / 60) as u8,
        second: (rem % 60) as u8,
    }
}

/// Days since 1970-01-01 to (year, month, day), proleptic Gregorian.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mentalcalc_core::{Difficulty, Operation, OperationSet};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "mentalcalc-storage-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn record(minute: u8) -> ArchiveRecord {
        ArchiveRecord {
            date: Timestamp::new(2014, 12, 11, 10, minute, 0).unwrap(),
            tasks: 20,
            correct: 15,
            incorrect: 5,
            minutes: 5,
            difficulty: Difficulty::Medium,
            operations: [Operation::Add, Operation::Multiply].into_iter().collect(),
        }
    }

    #[test]
    fn archive_round_trips_through_the_file() {
        let dir = temp_dir("archive");
        let storage = Storage::new(&dir);
        storage.append_record(&record(30)).unwrap();
        storage.append_record(&record(31)).unwrap();
        let load = storage.load_archive();
        assert_eq!(load.skipped, 0);
        assert_eq!(load.records, vec![record(30), record(31)]);
        storage.clear_archive().unwrap();
        assert_eq!(storage.load_archive(), ArchiveLoad::default());
        storage.clear_archive().unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn malformed_archive_lines_are_skipped_and_counted() {
        let text = "2014/12/11_10:30:00 20 15 5 5 2 1 0 0 0\n\
                    garbage line\n\
                    \n\
                    2014/12/11_10:31:00 20 16 5 5 2 1 0 0 0\n\
                    2014/12/11_10:32:00 20 15 5 5 2 1 0 0 0\r\n";
        let load = parse_archive(text.as_bytes());
        assert_eq!(load.records.len(), 2);
        assert_eq!(load.skipped, 2);
        assert_eq!(load.records[1].date.minute, 32);
    }

    #[test]
    fn invalid_utf8_lines_are_skipped_instead_of_failing() {
        let mut bytes = b"2014/12/11_10:30:00 20 15 5 5 2 1 0 0 0\n".to_vec();
        bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
        bytes.extend_from_slice(b"2014/12/11_10:31:00 20 15 5 5 2 1 0 0 0\n");
        let load = parse_archive(&bytes);
        assert_eq!(load.records.len(), 2);
        assert_eq!(load.skipped, 1);
    }

    #[test]
    fn append_after_a_missing_newline_keeps_records_apart() {
        let dir = temp_dir("newline");
        fs::write(
            dir.join(ARCHIVE_FILE),
            "2014/12/11_10:30:00 20 15 5 5 2 1 0 0 0",
        )
        .unwrap();
        Storage::new(&dir).append_record(&record(31)).unwrap();
        let load = Storage::new(&dir).load_archive();
        assert_eq!(load.records.len(), 2);
        assert_eq!(load.skipped, 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn settings_round_trip_with_language() {
        let dir = temp_dir("settings");
        let storage = Storage::new(&dir);
        assert_eq!(storage.load_settings(), StoredSettings::default());
        let stored = StoredSettings {
            settings: Settings {
                duration_minutes: 13,
                difficulty: Difficulty::Hard,
                operations: [Operation::Divide].into_iter().collect::<OperationSet>(),
            },
            language: Language::English,
        };
        storage.save_settings(&stored).unwrap();
        assert_eq!(storage.load_settings(), stored);
        assert!(!dir.join("settings.txt.tmp").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unknown_language_code_falls_back_to_default() {
        let parsed = parse_settings("session_duration=7\nlanguage=klingon\n");
        assert_eq!(parsed.language, Language::default());
        assert_eq!(parsed.settings.duration_minutes, 7);
    }

    #[test]
    fn unix_seconds_convert_to_utc_calendar_fields() {
        assert_eq!(
            timestamp_from_unix(0),
            Timestamp::new(1970, 1, 1, 0, 0, 0).unwrap()
        );
        assert_eq!(
            timestamp_from_unix(1_000_000_000),
            Timestamp::new(2001, 9, 9, 1, 46, 40).unwrap()
        );
        assert_eq!(
            timestamp_from_unix(1_700_000_000),
            Timestamp::new(2023, 11, 14, 22, 13, 20).unwrap()
        );
        // 2000 is a leap year; 2000-03-01 00:00:00 UTC is 951868800.
        assert_eq!(
            timestamp_from_unix(951_868_800),
            Timestamp::new(2000, 3, 1, 0, 0, 0).unwrap()
        );
    }
}
