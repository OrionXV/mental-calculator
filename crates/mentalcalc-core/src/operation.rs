use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Add,
    Subtract,
    Divide,
    Multiply,
}

impl Operation {
    /// Ordered by code, which is also the order used for random selection.
    pub const ALL: [Operation; 4] = [Self::Add, Self::Subtract, Self::Divide, Self::Multiply];

    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Add),
            1 => Some(Self::Subtract),
            2 => Some(Self::Divide),
            3 => Some(Self::Multiply),
            _ => None,
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::Add => 0,
            Self::Subtract => 1,
            Self::Divide => 2,
            Self::Multiply => 3,
        }
    }

    pub fn symbol(self) -> char {
        match self {
            Self::Add => '+',
            Self::Subtract => '\u{2212}',
            Self::Divide => '\u{00F7}',
            Self::Multiply => '\u{00D7}',
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// Enabled operations. Defaults to addition and subtraction, as in the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationSet {
    enabled: [bool; 4],
}

impl OperationSet {
    pub const fn empty() -> Self {
        Self {
            enabled: [false; 4],
        }
    }

    pub fn set(&mut self, op: Operation, on: bool) {
        self.enabled[usize::from(op.code())] = on;
    }

    pub fn contains(&self, op: Operation) -> bool {
        self.enabled[usize::from(op.code())]
    }

    pub fn is_empty(&self) -> bool {
        !self.enabled.iter().any(|&on| on)
    }

    pub fn iter(self) -> impl Iterator<Item = Operation> {
        Operation::ALL
            .into_iter()
            .filter(move |&op| self.contains(op))
    }
}

impl Default for OperationSet {
    fn default() -> Self {
        [Operation::Add, Operation::Subtract].into_iter().collect()
    }
}

impl FromIterator<Operation> for OperationSet {
    fn from_iter<I: IntoIterator<Item = Operation>>(iter: I) -> Self {
        let mut set = Self::empty();
        for op in iter {
            set.set(op, true);
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_match_original_operations() {
        assert_eq!(Operation::Add.code(), 0);
        assert_eq!(Operation::Subtract.code(), 1);
        assert_eq!(Operation::Divide.code(), 2);
        assert_eq!(Operation::Multiply.code(), 3);
        for op in Operation::ALL {
            assert_eq!(Operation::from_code(op.code()), Some(op));
        }
        assert_eq!(Operation::from_code(4), None);
    }

    #[test]
    fn display_symbols_use_unicode_minus_and_division_sign() {
        assert_eq!(Operation::Add.to_string(), "+");
        assert_eq!(Operation::Subtract.to_string(), "\u{2212}");
        assert_eq!(Operation::Divide.to_string(), "\u{00F7}");
        assert_eq!(Operation::Multiply.to_string(), "\u{00D7}");
    }

    #[test]
    fn default_set_is_addition_and_subtraction() {
        let set = OperationSet::default();
        assert!(set.contains(Operation::Add));
        assert!(set.contains(Operation::Subtract));
        assert!(!set.contains(Operation::Divide));
        assert!(!set.contains(Operation::Multiply));
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            vec![Operation::Add, Operation::Subtract]
        );
    }

    #[test]
    fn empty_set_reports_empty_and_iterates_nothing() {
        let mut set = OperationSet::empty();
        assert!(set.is_empty());
        assert_eq!(set.iter().count(), 0);
        set.set(Operation::Multiply, true);
        assert!(!set.is_empty());
        set.set(Operation::Multiply, false);
        assert!(set.is_empty());
    }
}
