use std::fmt;

use rand::Rng;

use crate::difficulty::Difficulty;
use crate::operation::{Operation, OperationSet};

/// One problem as shown to the user. For division `first` is the product
/// shown as dividend, `second` is the divisor and `answer` is the quotient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Task {
    pub first: u32,
    pub second: u32,
    pub op: Operation,
    pub answer: u32,
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.first, self.op, self.second)
    }
}

/// Keeps the previous operand pair so the same pair is not drawn twice in a row.
#[derive(Debug, Clone)]
pub struct TaskGenerator {
    difficulty: Difficulty,
    operations: OperationSet,
    previous: Option<(u32, u32)>,
}

impl TaskGenerator {
    /// Returns `None` when no operation is enabled.
    pub fn new(difficulty: Difficulty, operations: OperationSet) -> Option<Self> {
        (!operations.is_empty()).then_some(Self {
            difficulty,
            operations,
            previous: None,
        })
    }

    pub fn next_task<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Task {
        let enabled: Vec<Operation> = self.operations.iter().collect();
        let op = enabled[rng.random_range(0..enabled.len())];
        let (a, b) = loop {
            let pair = (
                rng.random_range(self.difficulty.first_range()),
                rng.random_range(self.difficulty.second_range()),
            );
            if Some(pair) != self.previous {
                break pair;
            }
        };
        self.previous = Some((a, b));
        build(op, a, b)
    }
}

fn build(op: Operation, a: u32, b: u32) -> Task {
    match op {
        Operation::Add => Task {
            first: a,
            second: b,
            op,
            answer: a + b,
        },
        Operation::Subtract => {
            let (first, second) = if a >= b { (a, b) } else { (b, a) };
            Task {
                first,
                second,
                op,
                answer: first - second,
            }
        }
        Operation::Divide => Task {
            first: a * b,
            second: b,
            op,
            answer: a,
        },
        Operation::Multiply => Task {
            first: a,
            second: b,
            op,
            answer: a * b,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const SEEDS: u64 = 200;
    const DRAWS: usize = 50;

    fn generator(difficulty: Difficulty, ops: &[Operation]) -> TaskGenerator {
        let operations: OperationSet = ops.iter().copied().collect();
        TaskGenerator::new(difficulty, operations).unwrap()
    }

    fn all_tasks(difficulty: Difficulty, ops: &[Operation]) -> Vec<Task> {
        let mut tasks = Vec::new();
        for seed in 0..SEEDS {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut tg = generator(difficulty, ops);
            tasks.extend((0..DRAWS).map(|_| tg.next_task(&mut rng)));
        }
        tasks
    }

    #[test]
    fn operands_stay_in_difficulty_ranges() {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Medium,
            Difficulty::Hard,
            Difficulty::Expert,
        ] {
            let first = difficulty.first_range();
            let second = difficulty.second_range();
            for task in all_tasks(difficulty, &[Operation::Add, Operation::Multiply]) {
                assert!(first.contains(&task.first), "{difficulty:?}: {task}");
                assert!(second.contains(&task.second), "{difficulty:?}: {task}");
            }
            for task in all_tasks(difficulty, &[Operation::Subtract]) {
                let straight = first.contains(&task.first) && second.contains(&task.second);
                let swapped = second.contains(&task.first) && first.contains(&task.second);
                assert!(straight || swapped, "{difficulty:?}: {task}");
            }
            for task in all_tasks(difficulty, &[Operation::Divide]) {
                assert!(first.contains(&task.answer), "{difficulty:?}: {task}");
                assert!(second.contains(&task.second), "{difficulty:?}: {task}");
            }
        }
    }

    #[test]
    fn answers_are_exact() {
        for task in all_tasks(Difficulty::Expert, &Operation::ALL) {
            let exact = match task.op {
                Operation::Add => task.answer == task.first + task.second,
                Operation::Subtract => task.answer == task.first - task.second,
                Operation::Divide => task.first == task.answer * task.second,
                Operation::Multiply => task.answer == task.first * task.second,
            };
            assert!(exact, "wrong answer for {task}");
        }
    }

    #[test]
    fn subtraction_is_never_negative() {
        for task in all_tasks(Difficulty::Medium, &[Operation::Subtract]) {
            assert!(task.first >= task.second, "negative result: {task}");
            assert_eq!(task.answer, task.first - task.second);
        }
    }

    #[test]
    fn division_is_exact_with_product_as_dividend() {
        for task in all_tasks(Difficulty::Hard, &[Operation::Divide]) {
            assert_ne!(task.second, 0);
            assert_eq!(task.first % task.second, 0, "inexact: {task}");
            assert_eq!(
                task.first / task.second,
                task.answer,
                "wrong quotient: {task}"
            );
        }
    }

    #[test]
    fn same_pair_is_not_repeated_in_a_row() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut tg = generator(Difficulty::Easy, &[Operation::Multiply]);
        let mut previous = tg.next_task(&mut rng);
        for _ in 0..5000 {
            let next = tg.next_task(&mut rng);
            assert_ne!((previous.first, previous.second), (next.first, next.second));
            previous = next;
        }
    }

    #[test]
    fn repeat_rule_applies_to_division_operand_pair() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut tg = generator(Difficulty::Easy, &[Operation::Divide]);
        let mut previous = tg.next_task(&mut rng);
        for _ in 0..5000 {
            let next = tg.next_task(&mut rng);
            assert_ne!(
                (previous.answer, previous.second),
                (next.answer, next.second)
            );
            // Retried draws must not scale the quotient a second time.
            assert!(Difficulty::Easy.first_range().contains(&next.answer));
            previous = next;
        }
    }

    #[test]
    fn enabled_operations_are_all_drawn() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut tg = generator(Difficulty::Medium, &[Operation::Add, Operation::Divide]);
        let mut seen = [0usize; 4];
        for _ in 0..4000 {
            let task = tg.next_task(&mut rng);
            seen[usize::from(task.op.code())] += 1;
        }
        assert!(seen[usize::from(Operation::Add.code())] > 1000);
        assert!(seen[usize::from(Operation::Divide.code())] > 1000);
        assert_eq!(seen[usize::from(Operation::Subtract.code())], 0);
        assert_eq!(seen[usize::from(Operation::Multiply.code())], 0);
    }

    #[test]
    fn generation_is_deterministic_for_a_seed() {
        let mut a = StdRng::seed_from_u64(99);
        let mut b = StdRng::seed_from_u64(99);
        let mut gen_a = generator(Difficulty::Hard, &Operation::ALL);
        let mut gen_b = generator(Difficulty::Hard, &Operation::ALL);
        for _ in 0..100 {
            assert_eq!(gen_a.next_task(&mut a), gen_b.next_task(&mut b));
        }
    }

    #[test]
    fn generator_requires_an_enabled_operation() {
        assert!(TaskGenerator::new(Difficulty::Easy, OperationSet::empty()).is_none());
    }

    #[test]
    fn display_uses_symbols() {
        let multiply = Task {
            first: 12,
            second: 7,
            op: Operation::Multiply,
            answer: 84,
        };
        let divide = Task {
            first: 84,
            second: 7,
            op: Operation::Divide,
            answer: 12,
        };
        assert_eq!(multiply.to_string(), "12 \u{00D7} 7");
        assert_eq!(divide.to_string(), "84 \u{00F7} 7");
    }
}
