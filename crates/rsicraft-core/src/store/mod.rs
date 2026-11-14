//! `store`: Stigmergic State Engine on SQLite (WAL mode).
//!
//! Manages `tasks`, `experience_cards` (with BLAKE3 content hashes and Zstd
//! compressed code blobs), and `task_frontiers`, plus composite utility
//! calculation and stochastic parent selection.
//!
//! Every multi-statement mutation runs inside one transaction (`with_tx`), so a
//! failure half way through never leaves counters, utilities and the frontier
//! out of step with the cards table.

pub mod cas;
pub mod dataset;
pub mod utility;

#[cfg(test)]
mod tests;

use cas::{compress_code, decompress_code, hash_code};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;
use utility::{CandidateMetrics, UtilityCalculator, UtilityHyperparams};

pub const SCHEMA_SQL: &str = include_str!("../../../../schema.sql");

pub const VALID_OPERATORS: &[&str] = &["DRAFT", "IMPROVE", "DEBUG", "CROSSOVER"];
pub const VALID_STATUSES: &[&str] = &[
    "SUCCESS",
    "COMPILE_ERROR",
    "RUNTIME_ERROR",
    "TIMEOUT",
    "METRIC_PARSE_ERROR",
];

/// Default size of the materialized per-task frontier.
pub const DEFAULT_FRONTIER_SIZE: usize = 10;

/// How long a connection waits on a lock held by another rsicraft process
/// (desktop studio, CLI and MCP server can all share one database file).
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

fn snap_temp_path() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("rsicraft-snap-{}-{nanos}.db", std::process::id()))
}

fn sidecar(path: &Path, suffix: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}{suffix}", path.display()))
}

fn write_db_file(path: &Path, bytes: &[u8]) -> StoreResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let _ = std::fs::remove_file(sidecar(path, "-wal"));
    let _ = std::fs::remove_file(sidecar(path, "-shm"));
    std::fs::write(path, bytes)?;
    Ok(())
}

fn backup_conn(src: &Connection, dst: &mut Connection) -> StoreResult<()> {
    let backup = rusqlite::backup::Backup::new(src, dst)?;
    for _ in 0..10_000 {
        match backup.step(64)? {
            rusqlite::backup::StepResult::Done => return Ok(()),
            rusqlite::backup::StepResult::More => {}
            _ => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    Err(StoreError::Validation(
        "sqlite backup did not finish".into(),
    ))
}

const HYPERPARAMS_KEY: &str = "utility_hyperparams";

const CARD_COLUMNS: &str = "card_id, task_id, parent_a_id, parent_b_id, operator_used,
                        code_hash, code_zstd, execution_status, raw_output,
                        primary_score, delta_score, novelty_score, composite_utility,
                        execution_time_ms, children_count, runs_count, score_std,
                        model_generation, created_at";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
}

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskRecord {
    pub task_id: String,
    pub name: String,
    pub description: String,
    pub evaluator_script: String,
    pub active_model_gen: u32,
    pub active_model_alias: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperienceCard {
    pub card_id: String,
    pub task_id: String,
    pub parent_a_id: Option<String>,
    pub parent_b_id: Option<String>,
    pub operator_used: String,
    pub code_hash: String,
    pub code: String,
    pub execution_status: String,
    pub raw_output: String,
    pub primary_score: f64,
    pub delta_score: f64,
    pub novelty_score: f64,
    pub composite_utility: f64,
    pub execution_time_ms: i64,
    pub children_count: u32,
    pub runs_count: u32,
    pub score_std: f64,
    pub model_generation: u32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrontierEntry {
    pub rank: i64,
    pub card: ExperienceCard,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskBundle {
    pub version: u32,
    pub exported_at: i64,
    pub task: TaskRecord,
    pub cards: Vec<ExperienceCard>,
    pub frontier: Vec<FrontierEntry>,
}

/// Score and delta ranges used for min-max normalization.
///
/// When a task has at least one `SUCCESS` card the ranges cover successful
/// cards only. Failed cards carry a score of 0, and letting them set the lower
/// bound squeezes every working candidate into the top fraction of the range,
/// which removes almost all selection pressure between working candidates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ScoreBounds {
    pub min_score: f64,
    pub max_score: f64,
    pub min_delta: f64,
    pub max_delta: f64,
    pub total_cards: u32,
    pub success_cards: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreStatus {
    pub journal_mode: String,
    pub tasks_count: i64,
    pub cards_count: i64,
    pub success_cards_count: i64,
    pub frontier_entries_count: i64,
    pub unique_code_hashes: i64,
}

#[derive(Debug, Clone)]
pub struct NewCardInput<'a> {
    pub card_id: &'a str,
    pub task_id: &'a str,
    pub parent_a_id: Option<&'a str>,
    pub parent_b_id: Option<&'a str>,
    pub operator_used: &'a str,
    pub code: &'a str,
    pub execution_status: &'a str,
    pub raw_output: &'a str,
    pub primary_score: f64,
    pub score_std: f64,
    pub runs_count: u32,
    pub model_generation: u32,
    pub execution_time_ms: i64,
}

/// Numeric projection of a card: everything utility math and parent sampling
/// need, without decompressing the code blob.
struct CardMetricsRow {
    card_id: String,
    primary_score: f64,
    delta_score: f64,
    children_count: u32,
    has_parent: bool,
    success: bool,
    composite_utility: f64,
}

fn to_u32(v: i64) -> u32 {
    u32::try_from(v.max(0)).unwrap_or(u32::MAX)
}

fn hyperparams_valid(p: &UtilityHyperparams) -> bool {
    let all = [p.lambda_s, p.lambda_delta, p.lambda_nu];
    all.iter().all(|w| w.is_finite() && *w >= 0.0) && all.iter().sum::<f64>() > 0.0
}

pub struct Registry {
    conn: Connection,
    path: Option<std::path::PathBuf>,
}

impl Registry {
    pub fn open(path: &Path) -> StoreResult<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        let reg = Self {
            conn,
            path: Some(path.to_path_buf()),
        };
        reg.migrate()?;
        Ok(reg)
    }

    pub fn open_in_memory() -> StoreResult<Self> {
        let conn = Connection::open_in_memory()?;
        let reg = Self { conn, path: None };
        reg.migrate()?;
        Ok(reg)
    }

    #[must_use]
    pub fn db_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Consistent single-file copy of the store, including anything still in the WAL.
    pub fn snapshot_bytes(&self) -> StoreResult<Vec<u8>> {
        let mut dst = Connection::open_in_memory()?;
        backup_conn(&self.conn, &mut dst)?;
        let path = snap_temp_path();
        {
            let mut file = Connection::open(&path)?;
            backup_conn(&dst, &mut file)?;
        }
        let bytes = std::fs::read(&path)?;
        let _ = std::fs::remove_file(&path);
        Ok(bytes)
    }

    /// Replace this connection with `bytes`. `path` is where the restored store lives.
    /// `None` keeps it in memory. The previous connection is dropped first so a file
    /// can be overwritten on Windows.
    pub fn restore_bytes(
        &mut self,
        bytes: &[u8],
        path: Option<std::path::PathBuf>,
    ) -> StoreResult<()> {
        let dummy = Connection::open_in_memory()?;
        let old = std::mem::replace(&mut self.conn, dummy);
        drop(old);
        let opened = match path {
            Some(p) => {
                write_db_file(&p, bytes)?;
                Self::open(&p)
            }
            None => Self::open_from_bytes(bytes),
        };
        match opened {
            Ok(reg) => {
                *self = reg;
                Ok(())
            }
            Err(e) => {
                self.path = None;
                let _ = self.migrate();
                Err(e)
            }
        }
    }

    fn open_from_bytes(bytes: &[u8]) -> StoreResult<Self> {
        let path = snap_temp_path();
        std::fs::write(&path, bytes)?;
        let src = Connection::open(&path)?;
        let mut dst = Connection::open_in_memory()?;
        let copied = backup_conn(&src, &mut dst);
        drop(src);
        let _ = std::fs::remove_file(&path);
        copied?;
        let reg = Self {
            conn: dst,
            path: None,
        };
        reg.migrate()?;
        Ok(reg)
    }

    pub fn checkpoint(&self) -> StoreResult<()> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    pub fn migrate(&self) -> StoreResult<()> {
        self.conn.execute_batch(SCHEMA_SQL)?;
        let _ = self.conn.execute(
            "ALTER TABLE tasks ADD COLUMN active_model_gen INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE tasks ADD COLUMN active_model_alias TEXT NOT NULL DEFAULT 'frontis'",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE experience_cards ADD COLUMN runs_count INTEGER NOT NULL DEFAULT 1",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE experience_cards ADD COLUMN score_std REAL NOT NULL DEFAULT 0.0",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE experience_cards ADD COLUMN model_generation INTEGER NOT NULL DEFAULT 0",
            [],
        );
        Ok(())
    }

    /// Run `f` inside one transaction. Re-entrant: when a transaction is already
    /// open on this connection, `f` simply joins it.
    fn with_tx<T>(&self, f: impl FnOnce() -> StoreResult<T>) -> StoreResult<T> {
        if !self.conn.is_autocommit() {
            return f();
        }
        let tx = self.conn.unchecked_transaction()?;
        let out = f()?;
        tx.commit()?;
        Ok(out)
    }

    pub fn status(&self) -> StoreResult<StoreStatus> {
        let journal_mode: String = self
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
        let tasks_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))?;
        let (cards_count, success_cards_count, unique_code_hashes): (i64, i64, i64) =
            self.conn.query_row(
                "SELECT COUNT(*),
                        COALESCE(SUM(execution_status = 'SUCCESS'), 0),
                        COUNT(DISTINCT code_hash)
                 FROM experience_cards",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
        let frontier_entries_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM task_frontiers", [], |r| r.get(0))?;

        Ok(StoreStatus {
            journal_mode: journal_mode.to_uppercase(),
            tasks_count,
            cards_count,
            success_cards_count,
            frontier_entries_count,
            unique_code_hashes,
        })
    }

    /// Load the persisted composite-utility weights, falling back to the
    /// defaults when none were saved or the stored value is unusable.
    #[must_use]
    pub fn load_hyperparams(&self) -> UtilityHyperparams {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![HYPERPARAMS_KEY],
                |r| r.get(0),
            )
            .optional()
            .unwrap_or(None);
        raw.and_then(|s| serde_json::from_str::<UtilityHyperparams>(&s).ok())
            .filter(hyperparams_valid)
            .unwrap_or_default()
    }

    /// Persist the composite-utility weights and re-rank every task with them,
    /// so the new weights take effect immediately and survive a restart.
    pub fn save_hyperparams(&self, hp: &UtilityHyperparams) -> StoreResult<()> {
        if !hyperparams_valid(hp) {
            return Err(StoreError::Validation(
                "utility weights must be finite, non-negative and sum to more than zero".into(),
            ));
        }
        let encoded =
            serde_json::to_string(hp).map_err(|e| StoreError::Validation(e.to_string()))?;
        self.with_tx(|| {
            self.conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![HYPERPARAMS_KEY, encoded],
            )?;
            let mut stmt = self.conn.prepare("SELECT task_id FROM tasks")?;
            let task_ids: Vec<String> = stmt
                .query_map([], |r| r.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            for task_id in &task_ids {
                self.recompute_task_utilities(task_id, hp)?;
                self.refresh_task_frontier(task_id, DEFAULT_FRONTIER_SIZE)?;
            }
            Ok(())
        })
    }

    pub fn upsert_task(
        &self,
        task_id: &str,
        name: &str,
        description: &str,
        evaluator_script: &str,
    ) -> StoreResult<TaskRecord> {
        let task_id = task_id.trim();
        let name = name.trim();
        if task_id.is_empty() || task_id.len() > 128 {
            return Err(StoreError::Validation(
                "task_id must be 1..=128 characters".into(),
            ));
        }
        if name.is_empty() {
            return Err(StoreError::Validation("task name cannot be empty".into()));
        }
        if description.trim().is_empty() {
            return Err(StoreError::Validation(
                "task description cannot be empty".into(),
            ));
        }
        if evaluator_script.trim().is_empty() {
            return Err(StoreError::Validation(
                "evaluator_script cannot be empty".into(),
            ));
        }

        self.conn.execute(
            "INSERT INTO tasks (task_id, name, description, evaluator_script)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(task_id) DO UPDATE SET
                 name = excluded.name,
                 description = excluded.description,
                 evaluator_script = excluded.evaluator_script",
            params![task_id, name, description, evaluator_script],
        )?;

        self.get_task(task_id)?
            .ok_or_else(|| StoreError::NotFound(format!("task {task_id}")))
    }

    pub fn set_task_active_model(&self, task_id: &str, gen: u32, alias: &str) -> StoreResult<()> {
        let rows = self.conn.execute(
            "UPDATE tasks SET active_model_gen = ?1, active_model_alias = ?2 WHERE task_id = ?3",
            params![gen, alias, task_id],
        )?;
        if rows == 0 {
            return Err(StoreError::NotFound(format!("task {task_id}")));
        }
        Ok(())
    }

    pub fn get_task(&self, task_id: &str) -> StoreResult<Option<TaskRecord>> {
        self.conn
            .query_row(
                "SELECT task_id, name, description, evaluator_script, active_model_gen, active_model_alias, created_at
                 FROM tasks WHERE task_id = ?1",
                params![task_id],
                |r| {
                    let gen_i64: i64 = r.get(4)?;
                    Ok(TaskRecord {
                        task_id: r.get(0)?,
                        name: r.get(1)?,
                        description: r.get(2)?,
                        evaluator_script: r.get(3)?,
                        active_model_gen: to_u32(gen_i64),
                        active_model_alias: r.get(5)?,
                        created_at: r.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn list_tasks(&self) -> StoreResult<Vec<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT task_id, name, description, evaluator_script, active_model_gen, active_model_alias, created_at
             FROM tasks ORDER BY created_at ASC, task_id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            let gen_i64: i64 = r.get(4)?;
            Ok(TaskRecord {
                task_id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                evaluator_script: r.get(3)?,
                active_model_gen: to_u32(gen_i64),
                active_model_alias: r.get(5)?,
                created_at: r.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_task_score_bounds(&self, task_id: &str) -> StoreResult<ScoreBounds> {
        let (min_s, max_s, min_d, max_d, total, success): (f64, f64, f64, f64, i64, i64) =
            self.conn.query_row(
                "SELECT
                    COALESCE(MIN(CASE WHEN execution_status = 'SUCCESS' THEN primary_score END),
                             MIN(primary_score), 0.0),
                    COALESCE(MAX(CASE WHEN execution_status = 'SUCCESS' THEN primary_score END),
                             MAX(primary_score), 1.0),
                    COALESCE(MIN(CASE WHEN execution_status = 'SUCCESS' THEN delta_score END),
                             MIN(delta_score), 0.0),
                    COALESCE(MAX(CASE WHEN execution_status = 'SUCCESS' THEN delta_score END),
                             MAX(delta_score), 1.0),
                    COUNT(*),
                    COALESCE(SUM(execution_status = 'SUCCESS'), 0)
                 FROM experience_cards WHERE task_id = ?1",
                params![task_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )?;
        Ok(ScoreBounds {
            min_score: min_s,
            max_score: max_s,
            min_delta: min_d,
            max_delta: max_d,
            total_cards: to_u32(total),
            success_cards: to_u32(success),
        })
    }

    fn decode_card_row(row: &rusqlite::Row<'_>) -> Result<ExperienceCard, rusqlite::Error> {
        let blob: Vec<u8> = row.get(6)?;
        let code = decompress_code(&blob).unwrap_or_else(|e| format!("# <zstd error: {e}>"));
        let children_i64: i64 = row.get(14)?;
        let runs_i64: i64 = row.get(15)?;
        let gen_i64: i64 = row.get(17)?;
        Ok(ExperienceCard {
            card_id: row.get(0)?,
            task_id: row.get(1)?,
            parent_a_id: row.get(2)?,
            parent_b_id: row.get(3)?,
            operator_used: row.get(4)?,
            code_hash: row.get(5)?,
            code,
            execution_status: row.get(7)?,
            raw_output: row.get(8)?,
            primary_score: row.get(9)?,
            delta_score: row.get(10)?,
            novelty_score: row.get(11)?,
            composite_utility: row.get(12)?,
            execution_time_ms: row.get(13)?,
            children_count: to_u32(children_i64),
            runs_count: to_u32(runs_i64),
            score_std: row.get(16)?,
            model_generation: to_u32(gen_i64),
            created_at: row.get(18)?,
        })
    }

    pub fn get_card(&self, card_id: &str) -> StoreResult<Option<ExperienceCard>> {
        self.conn
            .query_row(
                &format!("SELECT {CARD_COLUMNS} FROM experience_cards WHERE card_id = ?1"),
                params![card_id],
                Self::decode_card_row,
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn get_card_by_code_hash(
        &self,
        task_id: &str,
        code_hash: &str,
    ) -> StoreResult<Option<ExperienceCard>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {CARD_COLUMNS} FROM experience_cards
                     WHERE task_id = ?1 AND code_hash = ?2
                     ORDER BY created_at DESC LIMIT 1"
                ),
                params![task_id, code_hash],
                Self::decode_card_row,
            )
            .optional()
            .map_err(StoreError::from)
    }

    /// Owning task and primary score of a card, without touching the code blob.
    fn card_task_and_score(&self, card_id: &str) -> StoreResult<Option<(String, f64)>> {
        self.conn
            .query_row(
                "SELECT task_id, primary_score FROM experience_cards WHERE card_id = ?1",
                params![card_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(StoreError::from)
    }

    /// Resolve a parent reference and insist that it belongs to `task_id`.
    /// Cross-task lineage edges make task deletion and export unsound.
    fn parent_score(&self, role: &str, card_id: &str, task_id: &str) -> StoreResult<f64> {
        let (owner, score) = self
            .card_task_and_score(card_id)?
            .ok_or_else(|| StoreError::NotFound(format!("{role} card {card_id}")))?;
        if owner != task_id {
            return Err(StoreError::Validation(format!(
                "{role} card {card_id} belongs to task '{owner}', not '{task_id}'"
            )));
        }
        Ok(score)
    }

    pub fn record_experience_card(
        &self,
        input: &NewCardInput<'_>,
        params: &UtilityHyperparams,
    ) -> StoreResult<ExperienceCard> {
        if !VALID_OPERATORS.contains(&input.operator_used) {
            return Err(StoreError::Validation(format!(
                "invalid operator_used '{}'; expected one of {:?}",
                input.operator_used, VALID_OPERATORS
            )));
        }
        if !VALID_STATUSES.contains(&input.execution_status) {
            return Err(StoreError::Validation(format!(
                "invalid execution_status '{}'; expected one of {:?}",
                input.execution_status, VALID_STATUSES
            )));
        }
        if !input.primary_score.is_finite() {
            return Err(StoreError::Validation(
                "primary_score must be a finite number".into(),
            ));
        }
        let code_hash = hash_code(input.code);
        let code_zstd = compress_code(input.code).map_err(StoreError::Validation)?;

        self.with_tx(|| {
            if self.get_task(input.task_id)?.is_none() {
                return Err(StoreError::NotFound(format!("task {}", input.task_id)));
            }
            let parent_a_score = match input.parent_a_id {
                Some(pid) => Some(self.parent_score("parent_a", pid, input.task_id)?),
                None => None,
            };
            if let Some(pid_b) = input.parent_b_id {
                self.parent_score("parent_b", pid_b, input.task_id)?;
            }
            let raw_delta = parent_a_score.map_or(0.0, |ps| input.primary_score - ps);

            // Utility columns are placeholders here: the recompute below ranks
            // the new card together with every other card of the task.
            self.conn.execute(
                "INSERT INTO experience_cards (
                    card_id, task_id, parent_a_id, parent_b_id, operator_used,
                    code_hash, code_zstd, execution_status, raw_output,
                    primary_score, delta_score, novelty_score, composite_utility,
                    execution_time_ms, children_count, runs_count, score_std,
                    model_generation
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1.0, 0.0, ?12, 0, ?13, ?14, ?15)",
                params![
                    input.card_id,
                    input.task_id,
                    input.parent_a_id,
                    input.parent_b_id,
                    input.operator_used,
                    code_hash,
                    code_zstd,
                    input.execution_status,
                    input.raw_output,
                    input.primary_score,
                    raw_delta,
                    input.execution_time_ms,
                    input.runs_count,
                    input.score_std,
                    input.model_generation,
                ],
            )?;

            if let Some(pa) = input.parent_a_id {
                self.increment_children_count(pa)?;
            }
            if let Some(pb) = input.parent_b_id {
                if Some(pb) != input.parent_a_id {
                    self.increment_children_count(pb)?;
                }
            }

            self.recompute_task_utilities(input.task_id, params)?;
            self.refresh_task_frontier(input.task_id, DEFAULT_FRONTIER_SIZE)?;
            Ok(())
        })?;

        self.get_card(input.card_id)?
            .ok_or_else(|| StoreError::NotFound(format!("card {}", input.card_id)))
    }

    pub fn increment_children_count(&self, card_id: &str) -> StoreResult<()> {
        self.conn.execute(
            "UPDATE experience_cards
             SET children_count = children_count + 1,
                 novelty_score = 1.0 / (1.0 + (children_count + 1))
             WHERE card_id = ?1",
            params![card_id],
        )?;
        Ok(())
    }

    /// Numeric rows for one task, best utility first. `exclude_card_id` drops
    /// one card from the result.
    fn card_metrics(
        &self,
        task_id: &str,
        exclude_card_id: Option<&str>,
    ) -> StoreResult<Vec<CardMetricsRow>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT card_id, primary_score, delta_score, children_count,
                    parent_a_id IS NOT NULL, execution_status = 'SUCCESS', composite_utility
             FROM experience_cards
             WHERE task_id = ?1 AND (?2 IS NULL OR card_id != ?2)
             ORDER BY composite_utility DESC, created_at DESC, card_id ASC",
        )?;
        let rows = stmt.query_map(params![task_id, exclude_card_id], |r| {
            let children: i64 = r.get(3)?;
            Ok(CardMetricsRow {
                card_id: r.get(0)?,
                primary_score: r.get(1)?,
                delta_score: r.get(2)?,
                children_count: to_u32(children),
                has_parent: r.get(4)?,
                success: r.get(5)?,
                composite_utility: r.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Recompute `novelty_score` and `composite_utility` across all cards in a task
    /// whenever task score bounds or `children_count` shift.
    ///
    /// Works on numeric columns only (no code decompression) and has no card
    /// limit. A failed card earns no score term: its utility is the delta and
    /// novelty terms alone.
    pub fn recompute_task_utilities(
        &self,
        task_id: &str,
        params: &UtilityHyperparams,
    ) -> StoreResult<()> {
        self.with_tx(|| {
            let bounds = self.get_task_score_bounds(task_id)?;
            if bounds.total_cards == 0 {
                return Ok(());
            }
            let rows = self.card_metrics(task_id, None)?;
            let mut update = self.conn.prepare_cached(
                "UPDATE experience_cards
                 SET novelty_score = ?1, composite_utility = ?2
                 WHERE card_id = ?3",
            )?;
            for row in &rows {
                let parent_score = if row.has_parent {
                    Some(row.primary_score - row.delta_score)
                } else {
                    None
                };
                let metrics = CandidateMetrics {
                    primary_score: row.primary_score,
                    parent_primary_score: parent_score,
                    children_count: row.children_count,
                };
                let util = UtilityCalculator::calculate(
                    &metrics,
                    bounds.min_score,
                    bounds.max_score,
                    bounds.min_delta,
                    bounds.max_delta,
                    params,
                );
                let composite = if row.success {
                    util.composite
                } else {
                    util.composite - params.lambda_s * util.s_norm
                };
                update.execute(params![util.novelty_score, composite, row.card_id])?;
            }
            Ok(())
        })
    }

    pub fn refresh_task_frontier(&self, task_id: &str, limit: usize) -> StoreResult<()> {
        let limit_i64 = i64::try_from(limit.max(1)).unwrap_or(10);
        self.with_tx(|| {
            self.conn.execute(
                "DELETE FROM task_frontiers WHERE task_id = ?1",
                params![task_id],
            )?;
            self.conn.execute(
                "INSERT INTO task_frontiers (task_id, card_id, rank)
                 SELECT task_id, card_id,
                        ROW_NUMBER() OVER (
                            ORDER BY primary_score DESC, composite_utility DESC,
                                     execution_time_ms ASC, card_id ASC
                        )
                 FROM experience_cards
                 WHERE task_id = ?1 AND execution_status = 'SUCCESS'
                 ORDER BY primary_score DESC, composite_utility DESC,
                          execution_time_ms ASC, card_id ASC
                 LIMIT ?2",
                params![task_id, limit_i64],
            )?;
            Ok(())
        })
    }

    pub fn get_frontier(&self, task_id: &str) -> StoreResult<Vec<FrontierEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.card_id, c.task_id, c.parent_a_id, c.parent_b_id, c.operator_used,
                    c.code_hash, c.code_zstd, c.execution_status, c.raw_output,
                    c.primary_score, c.delta_score, c.novelty_score, c.composite_utility,
                    c.execution_time_ms, c.children_count, c.runs_count, c.score_std,
                    c.model_generation, c.created_at, f.rank
             FROM task_frontiers f
             JOIN experience_cards c ON c.card_id = f.card_id
             WHERE f.task_id = ?1
             ORDER BY f.rank ASC",
        )?;
        let rows = stmt.query_map(params![task_id], |r| {
            let card = Self::decode_card_row(r)?;
            let rank: i64 = r.get(19)?;
            Ok(FrontierEntry { rank, card })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn list_cards(
        &self,
        task_id: &str,
        operator: Option<&str>,
        status: Option<&str>,
        limit: usize,
    ) -> StoreResult<Vec<ExperienceCard>> {
        let limit_i64 = i64::try_from(limit.clamp(1, 10_000)).unwrap_or(100);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CARD_COLUMNS}
             FROM experience_cards
             WHERE task_id = ?1
               AND (?2 IS NULL OR operator_used = ?2)
               AND (?3 IS NULL OR execution_status = ?3)
             ORDER BY composite_utility DESC, created_at DESC, card_id ASC
             LIMIT ?4"
        ))?;
        let rows = stmt.query_map(params![task_id, operator, status, limit_i64], |r| {
            Self::decode_card_row(r)
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Select a parent candidate for `IMPROVE`, `DEBUG`, or `CROSSOVER`.
    ///
    /// For `DEBUG` (`prefer_faulty`), faulty candidates are preferred; otherwise
    /// working candidates are preferred. Each falls back to the whole pool when
    /// its preferred group is empty. Selection is a composite-utility roulette.
    ///
    /// With `exclude_card_id` set, that card is never returned: when it is the
    /// only card of the task the result is `None`, so `CROSSOVER` can never be
    /// handed the same parent twice.
    ///
    /// Only numeric columns are scanned; the code blob is decompressed for the
    /// one selected card.
    pub fn sample_parent_by_utility(
        &self,
        task_id: &str,
        prefer_faulty: bool,
        exclude_card_id: Option<&str>,
    ) -> StoreResult<Option<ExperienceCard>> {
        let pool = self.card_metrics(task_id, exclude_card_id)?;
        if pool.is_empty() {
            return Ok(None);
        }

        let preferred: Vec<&CardMetricsRow> =
            pool.iter().filter(|c| c.success != prefer_faulty).collect();
        let candidates: Vec<&CardMetricsRow> = if preferred.is_empty() {
            pool.iter().collect()
        } else {
            preferred
        };

        // Deterministic-yet-varied stochastic roulette wheel seeded by BLAKE3 over
        // the current card count + sum of children counts so identical state yields
        // reproducible selection while advancing state explores the frontier.
        let total_children: u64 = candidates.iter().map(|c| u64::from(c.children_count)).sum();
        let seed_input = format!(
            "{task_id}:{}:{total_children}:{}",
            candidates.len(),
            exclude_card_id.unwrap_or("")
        );
        let digest = blake3::hash(seed_input.as_bytes());
        let bytes = digest.as_bytes();
        let rand_u64 = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        #[allow(clippy::cast_precision_loss)]
        let unit = (rand_u64 as f64) / (u64::MAX as f64);

        let weights: Vec<f64> = candidates
            .iter()
            .map(|c| c.composite_utility.max(0.05))
            .collect();
        let weight_sum: f64 = weights.iter().sum();

        let target = unit * weight_sum;
        let mut accum = 0.0;
        let mut chosen = candidates.first().map(|c| c.card_id.as_str());
        for (card, w) in candidates.iter().zip(weights.iter()) {
            accum += *w;
            if accum >= target {
                chosen = Some(card.card_id.as_str());
                break;
            }
        }

        match chosen {
            Some(card_id) => self.get_card(card_id),
            None => Ok(None),
        }
    }

    /// Delete a task and all its associated frontier entries and experience cards.
    pub fn delete_task(&self, task_id: &str) -> StoreResult<bool> {
        self.with_tx(|| {
            if self.get_task(task_id)?.is_none() {
                return Ok(false);
            }
            // Frontier rows and lineage edges that point at this task's cards,
            // including any written from another task by an older build.
            self.conn.execute(
                "DELETE FROM task_frontiers
                 WHERE task_id = ?1
                    OR card_id IN (SELECT card_id FROM experience_cards WHERE task_id = ?1)",
                params![task_id],
            )?;
            self.conn.execute(
                "UPDATE experience_cards SET parent_a_id = NULL
                 WHERE parent_a_id IN (SELECT card_id FROM experience_cards WHERE task_id = ?1)",
                params![task_id],
            )?;
            self.conn.execute(
                "UPDATE experience_cards SET parent_b_id = NULL
                 WHERE parent_b_id IN (SELECT card_id FROM experience_cards WHERE task_id = ?1)",
                params![task_id],
            )?;
            self.conn.execute(
                "DELETE FROM experience_cards WHERE task_id = ?1",
                params![task_id],
            )?;
            let affected = self
                .conn
                .execute("DELETE FROM tasks WHERE task_id = ?1", params![task_id])?;
            Ok(affected > 0)
        })
    }

    /// Prune non-frontier experience cards for a task, keeping at least `keep_frontier` top cards.
    ///
    /// Surviving cards keep their `children_count`: it records how often a card
    /// was already expanded, which pruning its children does not undo.
    pub fn prune_cards(&self, task_id: &str, keep_frontier: usize) -> StoreResult<usize> {
        const DOOMED: &str = "SELECT card_id FROM experience_cards
             WHERE task_id = ?1
               AND card_id NOT IN (SELECT card_id FROM task_frontiers WHERE task_id = ?1)";
        self.with_tx(|| {
            self.refresh_task_frontier(task_id, keep_frontier)?;
            self.conn.execute(
                &format!(
                    "UPDATE experience_cards SET parent_a_id = NULL WHERE parent_a_id IN ({DOOMED})"
                ),
                params![task_id],
            )?;
            self.conn.execute(
                &format!(
                    "UPDATE experience_cards SET parent_b_id = NULL WHERE parent_b_id IN ({DOOMED})"
                ),
                params![task_id],
            )?;
            let pruned = self.conn.execute(
                &format!("DELETE FROM experience_cards WHERE card_id IN ({DOOMED})"),
                params![task_id],
            )?;
            Ok(pruned)
        })
    }

    /// Export a complete task lineage bundle (task, CAS cards, and frontier) to JSON Value.
    pub fn export_task_bundle(&self, task_id: &str) -> StoreResult<Value> {
        let task = self
            .get_task(task_id)?
            .ok_or_else(|| StoreError::NotFound(format!("task {task_id}")))?;
        let cards = self.list_cards(task_id, None, None, 10_000)?;
        let frontier = self.get_frontier(task_id)?;
        let bundle = TaskBundle {
            version: 1,
            exported_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| i64::try_from(d.as_secs()).unwrap_or(0))
                .unwrap_or(0),
            task,
            cards,
            frontier,
        };
        serde_json::to_value(&bundle).map_err(|e| StoreError::Validation(e.to_string()))
    }

    /// Import a complete task lineage bundle into SQLite.
    ///
    /// Runs in one transaction with deferred foreign keys, so cards may arrive in
    /// any order and a rejected bundle leaves the store untouched. Every card is
    /// filed under the bundle's own task, parent references that resolve to
    /// nothing are cleared, and the frontier is rebuilt from the imported cards
    /// instead of being trusted from the file. Returns the number of cards written.
    pub fn import_task_bundle(
        &self,
        bundle_val: &Value,
        params: &UtilityHyperparams,
    ) -> StoreResult<usize> {
        let bundle: TaskBundle = serde_json::from_value(bundle_val.clone())
            .map_err(|e| StoreError::Validation(format!("invalid task bundle JSON: {e}")))?;
        let task_id = bundle.task.task_id.trim().to_string();

        for c in &bundle.cards {
            if !VALID_OPERATORS.contains(&c.operator_used.as_str()) {
                return Err(StoreError::Validation(format!(
                    "card {}: invalid operator_used '{}'",
                    c.card_id, c.operator_used
                )));
            }
            if !VALID_STATUSES.contains(&c.execution_status.as_str()) {
                return Err(StoreError::Validation(format!(
                    "card {}: invalid execution_status '{}'",
                    c.card_id, c.execution_status
                )));
            }
            if !c.primary_score.is_finite() || !c.delta_score.is_finite() {
                return Err(StoreError::Validation(format!(
                    "card {}: scores must be finite numbers",
                    c.card_id
                )));
            }
        }

        self.with_tx(|| {
            self.conn.execute("PRAGMA defer_foreign_keys = ON", [])?;
            self.upsert_task(
                &task_id,
                &bundle.task.name,
                &bundle.task.description,
                &bundle.task.evaluator_script,
            )?;

            let mut written = 0;
            for c in &bundle.cards {
                let code_hash = hash_code(&c.code);
                let code_zstd = compress_code(&c.code).map_err(StoreError::Validation)?;
                written += self.conn.execute(
                    "INSERT INTO experience_cards (
                        card_id, task_id, parent_a_id, parent_b_id, operator_used,
                        code_hash, code_zstd, execution_status, raw_output,
                        primary_score, delta_score, novelty_score, composite_utility,
                        execution_time_ms, children_count, runs_count, score_std,
                        model_generation, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
                     ON CONFLICT(card_id) DO UPDATE SET
                        composite_utility = excluded.composite_utility,
                        primary_score = excluded.primary_score,
                        delta_score = excluded.delta_score,
                        novelty_score = excluded.novelty_score,
                        children_count = excluded.children_count,
                        runs_count = excluded.runs_count,
                        score_std = excluded.score_std,
                        model_generation = excluded.model_generation
                     WHERE experience_cards.task_id = excluded.task_id",
                    params![
                        c.card_id,
                        task_id,
                        c.parent_a_id,
                        c.parent_b_id,
                        c.operator_used,
                        code_hash,
                        code_zstd,
                        c.execution_status,
                        c.raw_output,
                        c.primary_score,
                        c.delta_score,
                        c.novelty_score,
                        c.composite_utility,
                        c.execution_time_ms,
                        c.children_count,
                        c.runs_count,
                        c.score_std,
                        c.model_generation,
                        c.created_at,
                    ],
                )?;
            }

            // A parent may only be a card of the same task that actually exists.
            for column in ["parent_a_id", "parent_b_id"] {
                self.conn.execute(
                    &format!(
                        "UPDATE experience_cards SET {column} = NULL
                         WHERE task_id = ?1 AND {column} IS NOT NULL
                           AND {column} NOT IN (
                               SELECT card_id FROM experience_cards WHERE task_id = ?1
                           )"
                    ),
                    params![task_id],
                )?;
            }

            self.recompute_task_utilities(&task_id, params)?;
            self.refresh_task_frontier(&task_id, DEFAULT_FRONTIER_SIZE)?;
            Ok(written)
        })
    }
}
