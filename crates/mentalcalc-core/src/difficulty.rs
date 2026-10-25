use std::ops::RangeInclusive;

/// Difficulty used whenever no value is stored. The original app generated
/// problems with 1 but archived rows with 3 when the setting was unset.
pub const DEFAULT_DIFFICULTY: Difficulty = Difficulty::Medium;

/// Operand digit ranges. Codes match the original `difficulty` preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

impl Difficulty {
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Easy),
            2 => Some(Self::Medium),
            3 => Some(Self::Hard),
            4 => Some(Self::Expert),
            _ => None,
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::Easy => 1,
            Self::Medium => 2,
            Self::Hard => 3,
            Self::Expert => 4,
        }
    }

    pub fn first_range(self) -> RangeInclusive<u32> {
        match self {
            Self::Easy => 2..=9,
            Self::Medium | Self::Hard => 10..=99,
            Self::Expert => 100..=999,
        }
    }

    pub fn second_range(self) -> RangeInclusive<u32> {
        match self {
            Self::Easy | Self::Medium => 2..=9,
            Self::Hard => 10..=99,
            Self::Expert => 100..=999,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for code in 1..=4 {
            let difficulty = Difficulty::from_code(code).unwrap();
            assert_eq!(difficulty.code(), code);
        }
    }

    #[test]
    fn unknown_codes_are_rejected() {
        for code in [0, 5, 9, 255] {
            assert_eq!(Difficulty::from_code(code), None);
        }
    }

    #[test]
    fn ranges_match_original_table() {
        assert_eq!(Difficulty::Easy.first_range(), 2..=9);
        assert_eq!(Difficulty::Easy.second_range(), 2..=9);
        assert_eq!(Difficulty::Medium.first_range(), 10..=99);
        assert_eq!(Difficulty::Medium.second_range(), 2..=9);
        assert_eq!(Difficulty::Hard.first_range(), 10..=99);
        assert_eq!(Difficulty::Hard.second_range(), 10..=99);
        assert_eq!(Difficulty::Expert.first_range(), 100..=999);
        assert_eq!(Difficulty::Expert.second_range(), 100..=999);
    }

    #[test]
    fn default_difficulty_is_medium_code_two() {
        assert_eq!(DEFAULT_DIFFICULTY, Difficulty::Medium);
        assert_eq!(DEFAULT_DIFFICULTY.code(), 2);
    }
}
