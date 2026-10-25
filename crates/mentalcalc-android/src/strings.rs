//! English and Russian UI text. Russian is verbatim from the original app;
//! English is the gloss from `re/strings.md`.

use mentalcalc_core::{Difficulty, Grade, Operation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    English,
    Russian,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::English, Language::Russian];

    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Russian => "ru",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::English),
            "ru" => Some(Self::Russian),
            _ => None,
        }
    }

    /// Always shown in the language itself, so it stays readable in either mode.
    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Russian => "Русский",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    AppName,
    Start,
    Archive,
    SessionSettings,
    Settings,
    Back,
    Cancel,
    Clear,
    End,
    Duration,
    Difficulty,
    DifficultyEasy,
    DifficultyMedium,
    DifficultyHard,
    DifficultyExpert,
    Operations,
    OpAdd,
    OpSubtract,
    OpDivide,
    OpMultiply,
    Language,
    TimeLeft,
    Pause,
    Resume,
    Paused,
    Correct,
    Incorrect,
    Answer,
    Delete,
    Skipped,
    ResultTitle,
    ResultTotal,
    ResultCorrect,
    ResultIncorrect,
    ResultPercent,
    ResultGrade,
    ResultSpeed,
    SpeedUnit,
    GradeNoTasks,
    GradeVeryBad,
    GradeBad,
    GradeNormal,
    GradePerfect,
    ArchiveEmpty,
    ArchiveSessions,
    ArchiveSolved,
    ArchiveClear,
    ArchiveClearTitle,
    ArchiveClearMessage,
    EndTitle,
    EndMessage,
    ConfigureFirst,
    StorageError,
}

impl Key {
    pub const ALL: &'static [Key] = &[
        Key::AppName,
        Key::Start,
        Key::Archive,
        Key::SessionSettings,
        Key::Settings,
        Key::Back,
        Key::Cancel,
        Key::Clear,
        Key::End,
        Key::Duration,
        Key::Difficulty,
        Key::DifficultyEasy,
        Key::DifficultyMedium,
        Key::DifficultyHard,
        Key::DifficultyExpert,
        Key::Operations,
        Key::OpAdd,
        Key::OpSubtract,
        Key::OpDivide,
        Key::OpMultiply,
        Key::Language,
        Key::TimeLeft,
        Key::Pause,
        Key::Resume,
        Key::Paused,
        Key::Correct,
        Key::Incorrect,
        Key::Answer,
        Key::Delete,
        Key::Skipped,
        Key::ResultTitle,
        Key::ResultTotal,
        Key::ResultCorrect,
        Key::ResultIncorrect,
        Key::ResultPercent,
        Key::ResultGrade,
        Key::ResultSpeed,
        Key::SpeedUnit,
        Key::GradeNoTasks,
        Key::GradeVeryBad,
        Key::GradeBad,
        Key::GradeNormal,
        Key::GradePerfect,
        Key::ArchiveEmpty,
        Key::ArchiveSessions,
        Key::ArchiveSolved,
        Key::ArchiveClear,
        Key::ArchiveClearTitle,
        Key::ArchiveClearMessage,
        Key::EndTitle,
        Key::EndMessage,
        Key::ConfigureFirst,
        Key::StorageError,
    ];
}

/// Exhaustive on purpose: adding a key without both texts does not compile.
fn pair(key: Key) -> (&'static str, &'static str) {
    match key {
        Key::AppName => ("Mental Calculator", "Счет в уме"),
        Key::Start => ("Start session", "Старт"),
        Key::Archive => ("Session archive", "Архив сессий"),
        Key::SessionSettings => ("Session settings", "Настройки сессии"),
        Key::Settings => ("App settings", "Настройки приложения"),
        Key::Back => ("Back", "Назад"),
        Key::Cancel => ("Cancel", "Отмена"),
        Key::Clear => ("Clear", "Очистить"),
        Key::End => ("End", "Завершить"),
        Key::Duration => ("Counting time", "Время счета"),
        Key::Difficulty => ("Difficulty", "Сложность"),
        Key::DifficultyEasy => ("x and x", "x и x"),
        Key::DifficultyMedium => ("xx and x", "xx и x"),
        Key::DifficultyHard => ("xx and xx", "xx и xx"),
        Key::DifficultyExpert => ("xxx and xxx", "xxx и xxx"),
        Key::Operations => ("Operations", "Операции"),
        Key::OpAdd => ("Addition", "Сложение"),
        Key::OpSubtract => ("Subtraction", "Разность"),
        Key::OpDivide => ("Division", "Деление"),
        Key::OpMultiply => ("Multiplication", "Умножение"),
        Key::Language => ("Language", "Язык"),
        Key::TimeLeft => ("Time left", "Осталось"),
        Key::Pause => ("Pause", "Пауза"),
        Key::Resume => ("Resume", "Продолжить"),
        Key::Paused => ("Paused", "Пауза"),
        Key::Correct => ("Correct", "Правильно"),
        Key::Incorrect => ("Incorrect", "Неправильно"),
        Key::Answer => ("Answer", "Ответ"),
        Key::Delete => ("Del", "Стереть"),
        Key::Skipped => ("Skipped", "Пропущено"),
        Key::ResultTitle => ("Results", "Результаты"),
        Key::ResultTotal => ("Total solved", "Всего решено"),
        Key::ResultCorrect => ("Solved correctly", "Правильно решено"),
        Key::ResultIncorrect => ("Solved incorrectly", "Неправильно решено"),
        Key::ResultPercent => ("% correct", "% правильных"),
        Key::ResultGrade => ("Grade", "Оценка"),
        Key::ResultSpeed => ("Speed", "Скорость"),
        Key::SpeedUnit => ("examples/min", "прим/мин"),
        Key::GradeNoTasks => ("They did not even try", "Даже не старались"),
        Key::GradeVeryBad => ("Very bad", "Очень плохо"),
        Key::GradeBad => ("Bad", "Плохо"),
        Key::GradeNormal => ("Good", "Хорошо"),
        Key::GradePerfect => ("Excellent", "Великолепно"),
        Key::ArchiveEmpty => ("No sessions yet", "Статистика пустая"),
        Key::ArchiveSessions => ("Sessions played", "Всего сессий сыграно"),
        Key::ArchiveSolved => ("Examples solved", "Всего примеров решено"),
        Key::ArchiveClear => ("Clear archive", "Обнулить статистику"),
        Key::ArchiveClearTitle => ("Clear archive?", "Очистить архив?"),
        Key::ArchiveClearMessage => ("This action is irreversible!", "Это действие необратимо!"),
        Key::EndTitle => ("Ending the session!", "Завершение сессии!"),
        Key::EndMessage => (
            "If you end the session now, the data will not be saved!",
            "Если вы завершите сессию сейчас, данные не сохранятся!",
        ),
        Key::ConfigureFirst => ("First configure the session", "Сначала настройте сессию"),
        Key::StorageError => ("Storage error", "Ошибка сохранения"),
    }
}

pub fn text(lang: Language, key: Key) -> &'static str {
    let (english, russian) = pair(key);
    match lang {
        Language::English => english,
        Language::Russian => russian,
    }
}

pub fn difficulty(lang: Language, difficulty: Difficulty) -> &'static str {
    text(
        lang,
        match difficulty {
            Difficulty::Easy => Key::DifficultyEasy,
            Difficulty::Medium => Key::DifficultyMedium,
            Difficulty::Hard => Key::DifficultyHard,
            Difficulty::Expert => Key::DifficultyExpert,
        },
    )
}

pub fn operation(lang: Language, op: Operation) -> &'static str {
    text(
        lang,
        match op {
            Operation::Add => Key::OpAdd,
            Operation::Subtract => Key::OpSubtract,
            Operation::Divide => Key::OpDivide,
            Operation::Multiply => Key::OpMultiply,
        },
    )
}

pub fn grade(lang: Language, grade: Grade) -> &'static str {
    text(
        lang,
        match grade {
            Grade::NoTasks => Key::GradeNoTasks,
            Grade::VeryBad => Key::GradeVeryBad,
            Grade::Bad => Key::GradeBad,
            Grade::Normal => Key::GradeNormal,
            Grade::Perfect => Key::GradePerfect,
        },
    )
}

pub fn minutes(lang: Language, count: u32) -> String {
    match lang {
        Language::English => format!("{count} min"),
        Language::Russian => format!("{count} мин"),
    }
}

pub fn skipped_lines(lang: Language, count: usize) -> String {
    match lang {
        Language::English => format!("Unreadable archive lines skipped: {count}"),
        Language::Russian => format!("Пропущено нечитаемых строк архива: {count}"),
    }
}

pub fn skipped_task(lang: Language, problem: &str, answer: u32) -> String {
    format!("{}: {problem} = {answer}", text(lang, Key::Skipped))
}

pub fn storage_failure(lang: Language, detail: &str) -> String {
    format!("{}: {detail}", text(lang, Key::StorageError))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_has_text_in_both_languages() {
        for &key in Key::ALL {
            assert!(!text(Language::English, key).is_empty(), "{key:?}");
            assert!(!text(Language::Russian, key).is_empty(), "{key:?}");
        }
    }

    #[test]
    fn lookup_picks_the_requested_language() {
        assert_eq!(text(Language::English, Key::Start), "Start session");
        assert_eq!(text(Language::Russian, Key::Start), "Старт");
        assert_eq!(
            text(Language::Russian, Key::Settings),
            "Настройки приложения"
        );
    }

    #[test]
    fn difficulty_and_operation_labels_follow_original_table() {
        assert_eq!(difficulty(Language::Russian, Difficulty::Easy), "x и x");
        assert_eq!(
            difficulty(Language::Russian, Difficulty::Expert),
            "xxx и xxx"
        );
        assert_eq!(operation(Language::English, Operation::Divide), "Division");
        assert_eq!(
            operation(Language::Russian, Operation::Subtract),
            "Разность"
        );
    }

    #[test]
    fn language_codes_round_trip() {
        for lang in Language::ALL {
            assert_eq!(Language::from_code(lang.code()), Some(lang));
        }
        assert_eq!(Language::from_code("de"), None);
        assert_eq!(Language::default(), Language::English);
    }

    #[test]
    fn formatted_strings_include_their_values() {
        assert_eq!(minutes(Language::English, 5), "5 min");
        assert_eq!(minutes(Language::Russian, 12), "12 мин");
        assert!(skipped_lines(Language::English, 3).ends_with(": 3"));
        assert_eq!(
            skipped_task(Language::English, "12 \u{00D7} 7", 84),
            "Skipped: 12 \u{00D7} 7 = 84"
        );
    }
}
