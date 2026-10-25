use crate::difficulty::{DEFAULT_DIFFICULTY, Difficulty};
use crate::operation::{Operation, OperationSet};

pub const MIN_DURATION_MINUTES: u32 = 1;
pub const MAX_DURATION_MINUTES: u32 = 20;
pub const DEFAULT_DURATION_MINUTES: u32 = 5;

const KEY_DURATION: &str = "session_duration";
const KEY_DIFFICULTY: &str = "difficulty";
const KEY_ADDITION: &str = "operation_addition";
const KEY_SUBTRACTION: &str = "operation_difference";
const KEY_MULTIPLICATION: &str = "operation_multiplication";
const KEY_DIVISION: &str = "operation_division";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub duration_minutes: u32,
    pub difficulty: Difficulty,
    pub operations: OperationSet,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            duration_minutes: DEFAULT_DURATION_MINUTES,
            difficulty: DEFAULT_DIFFICULTY,
            operations: OperationSet::default(),
        }
    }
}

impl Settings {
    /// Reads `key=value` lines. Unknown keys and lines without `=` are ignored.
    /// A malformed value resets only its own key to the default.
    pub fn parse(text: &str) -> Self {
        let defaults = Self::default();
        let mut settings = defaults.clone();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                KEY_DURATION => {
                    settings.duration_minutes =
                        parse_duration(value).unwrap_or(defaults.duration_minutes);
                }
                KEY_DIFFICULTY => {
                    settings.difficulty = value
                        .parse::<u8>()
                        .ok()
                        .and_then(Difficulty::from_code)
                        .unwrap_or(defaults.difficulty);
                }
                KEY_ADDITION => set_flag(&mut settings, &defaults, Operation::Add, value),
                KEY_SUBTRACTION => set_flag(&mut settings, &defaults, Operation::Subtract, value),
                KEY_MULTIPLICATION => {
                    set_flag(&mut settings, &defaults, Operation::Multiply, value)
                }
                KEY_DIVISION => set_flag(&mut settings, &defaults, Operation::Divide, value),
                _ => {}
            }
        }
        settings
    }

    pub fn format(&self) -> String {
        let flag = |op| self.operations.contains(op);
        format!(
            "{KEY_DURATION}={}\n{KEY_DIFFICULTY}={}\n{KEY_ADDITION}={}\n{KEY_SUBTRACTION}={}\n{KEY_MULTIPLICATION}={}\n{KEY_DIVISION}={}\n",
            self.duration_minutes,
            self.difficulty.code(),
            flag(Operation::Add),
            flag(Operation::Subtract),
            flag(Operation::Multiply),
            flag(Operation::Divide),
        )
    }
}

fn set_flag(settings: &mut Settings, defaults: &Settings, op: Operation, value: &str) {
    let on = parse_flag(value).unwrap_or(defaults.operations.contains(op));
    settings.operations.set(op, on);
}

fn parse_duration(value: &str) -> Option<u32> {
    value
        .parse::<u32>()
        .ok()
        .filter(|minutes| (MIN_DURATION_MINUTES..=MAX_DURATION_MINUTES).contains(minutes))
}

fn parse_flag(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_original_preferences() {
        let settings = Settings::default();
        assert_eq!(settings.duration_minutes, 5);
        assert_eq!(settings.difficulty, Difficulty::Medium);
        assert!(settings.operations.contains(Operation::Add));
        assert!(settings.operations.contains(Operation::Subtract));
        assert!(!settings.operations.contains(Operation::Multiply));
        assert!(!settings.operations.contains(Operation::Divide));
    }

    /// Regression: default difficulty is one constant across settings and archive.
    #[test]
    fn default_difficulty_constant_is_used_everywhere() {
        assert_eq!(Settings::default().difficulty, DEFAULT_DIFFICULTY);
        assert_eq!(Settings::parse("").difficulty, DEFAULT_DIFFICULTY);
        assert!(Settings::default().format().contains("\ndifficulty=2\n"));
    }

    #[test]
    fn empty_text_yields_defaults() {
        assert_eq!(Settings::parse(""), Settings::default());
    }

    #[test]
    fn format_then_parse_round_trips() {
        let settings = Settings {
            duration_minutes: 13,
            difficulty: Difficulty::Expert,
            operations: [Operation::Multiply, Operation::Divide]
                .into_iter()
                .collect(),
        };
        assert_eq!(Settings::parse(&settings.format()), settings);
    }

    #[test]
    fn malformed_values_fall_back_to_default_per_key() {
        let text = "session_duration=abc\n\
                    difficulty=7\n\
                    operation_addition=yes\n\
                    operation_multiplication=true";
        let settings = Settings::parse(text);
        assert_eq!(settings.duration_minutes, DEFAULT_DURATION_MINUTES);
        assert_eq!(settings.difficulty, DEFAULT_DIFFICULTY);
        assert!(settings.operations.contains(Operation::Add));
        assert!(settings.operations.contains(Operation::Multiply));
    }

    #[test]
    fn duration_outside_one_to_twenty_is_malformed() {
        assert_eq!(Settings::parse("session_duration=0").duration_minutes, 5);
        assert_eq!(Settings::parse("session_duration=21").duration_minutes, 5);
        assert_eq!(Settings::parse("session_duration=20").duration_minutes, 20);
        assert_eq!(Settings::parse("session_duration=1").duration_minutes, 1);
    }

    #[test]
    fn unknown_keys_and_lines_without_equals_are_ignored() {
        let text = "time=12:00\nno equals here\nsession_duration=10\nis_notification_active=true";
        let settings = Settings::parse(text);
        assert_eq!(settings.duration_minutes, 10);
        assert_eq!(settings.difficulty, DEFAULT_DIFFICULTY);
    }

    #[test]
    fn whitespace_around_keys_and_values_is_trimmed() {
        let settings = Settings::parse("  difficulty = 4  \n");
        assert_eq!(settings.difficulty, Difficulty::Expert);
    }

    #[test]
    fn parsed_empty_operation_set_is_kept_for_session_to_reject() {
        let settings = Settings::parse("operation_addition=false\noperation_difference=false");
        assert!(settings.operations.is_empty());
    }
}
