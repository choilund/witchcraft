pub mod claude_code;
pub mod codex;
pub mod gemini;

use anyhow::Result;
use witchcraft::DB;

/// An abstraction for fetching user/agent pair programming history.
#[allow(dead_code)]
pub trait Ingestor {
    /// Ingest available sessions/memories into the database.
    /// Returns the number of chunks/turns successfully embedded.
    fn ingest(&self, db: &mut DB) -> Result<usize>;
}
