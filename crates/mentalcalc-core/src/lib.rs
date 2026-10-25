//! Platform-independent core of the mental arithmetic trainer.
#![forbid(unsafe_code)]

pub mod archive;
pub mod difficulty;
pub mod operation;
pub mod session;
pub mod settings;
pub mod task;

pub use archive::{ArchiveError, ArchiveRecord, Timestamp};
pub use difficulty::{DEFAULT_DIFFICULTY, Difficulty};
pub use operation::{Operation, OperationSet};
pub use session::{Grade, Outcome, Phase, Session, SessionError, SessionSummary};
pub use settings::Settings;
pub use task::{Task, TaskGenerator};
