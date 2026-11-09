//! `store`: Stigmergic State Engine on SQLite (WAL mode).
//!
//! Manages `tasks`, `experience_cards` (with BLAKE3 content hashes and Zstd
//! compressed code blobs), and `task_frontiers`, plus composite utility
//! calculation and stochastic parent selection.

pub mod cas;
pub mod utility;

#[cfg(test)]
mod tests;

use cas::{compress_code, decompress_code, hash_code};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
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
    pub execution_time_ms: i64,
}

pub struct Registry {
    conn: Connection,
}

impl Registry {
    pub fn open(path: &Path) -> StoreResult<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        let reg = Self { conn };
        reg.migrate()?;
        Ok(reg)
    }

    pub fn open_in_memory() -> StoreResult<Self> {
        let conn = Connection::open_in_memory()?;
        let reg = Self { conn };
        reg.migrate()?;
        Ok(reg)
    }

    pub fn migrate(&self) -> StoreResult<()> {
        self.conn.execute_batch(SCHEMA_SQL)?;
        Ok(())
    }

    pub fn status(&self) -> StoreResult<StoreStatus> {
        let journal_mode: String = self
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
        let tasks_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))?;
        let cards_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM experience_cards", [], |r| r.get(0))?;
        let success_cards_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM experience_cards WHERE execution_status = 'SUCCESS'",
            [],
            |r| r.get(0),
        )?;
        let frontier_entries_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM task_frontiers", [], |r| r.get(0))?;
        let unique_code_hashes: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT code_hash) FROM experience_cards",
            [],
            |r| r.get(0),
        )?;

        Ok(StoreStatus {
            journal_mode: journal_mode.to_uppercase(),
            tasks_count,
            cards_count,
            success_cards_count,
            frontier_entries_count,
            unique_code_hashes,
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

    pub fn get_task(&self, task_id: &str) -> StoreResult<Option<TaskRecord>> {
        self.conn
            .query_row(
                "SELECT task_id, name, description, evaluator_script, created_at
                 FROM tasks WHERE task_id = ?1",
                params![task_id],
                |r| {
                    Ok(TaskRecord {
                        task_id: r.get(0)?,
                        name: r.get(1)?,
                        description: r.get(2)?,
                        evaluator_script: r.get(3)?,
                        created_at: r.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn list_tasks(&self) -> StoreResult<Vec<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT task_id, name, description, evaluator_script, created_at
             FROM tasks ORDER BY created_at ASC, task_id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(TaskRecord {
                task_id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                evaluator_script: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_task_score_bounds(&self, task_id: &str) -> StoreResult<ScoreBounds> {
        let (min_s, max_s, min_d, max_d, total): (f64, f64, f64, f64, i64) = self.conn.query_row(
            "SELECT
                    COALESCE(MIN(primary_score), 0.0),
                    COALESCE(MAX(primary_score), 1.0),
                    COALESCE(MIN(delta_score), 0.0),
                    COALESCE(MAX(delta_score), 1.0),
                    COUNT(*)
                 FROM experience_cards WHERE task_id = ?1",
            params![task_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )?;
        let success_cards: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM experience_cards
             WHERE task_id = ?1 AND execution_status = 'SUCCESS'",
            params![task_id],
            |r| r.get(0),
        )?;
        Ok(ScoreBounds {
            min_score: min_s,
            max_score: max_s,
            min_delta: min_d,
            max_delta: max_d,
            total_cards: u32::try_from(total.max(0)).unwrap_or(u32::MAX),
            success_cards: u32::try_from(success_cards.max(0)).unwrap_or(u32::MAX),
        })
    }

    fn decode_card_row(row: &rusqlite::Row<'_>) -> Result<ExperienceCard, rusqlite::Error> {
        let blob: Vec<u8> = row.get(6)?;
        let code = decompress_code(&blob).unwrap_or_else(|e| format!("# <zstd error: {e}>"));
        let children_i64: i64 = row.get(14)?;
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
            children_count: u32::try_from(children_i64.max(0)).unwrap_or(u32::MAX),
            created_at: row.get(15)?,
        })
    }

    pub fn get_card(&self, card_id: &str) -> StoreResult<Option<ExperienceCard>> {
        self.conn
            .query_row(
                "SELECT card_id, task_id, parent_a_id, parent_b_id, operator_used,
                        code_hash, code_zstd, execution_status, raw_output,
                        primary_score, delta_score, novelty_score, composite_utility,
                        execution_time_ms, children_count, created_at
                 FROM experience_cards WHERE card_id = ?1",
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
                "SELECT card_id, task_id, parent_a_id, parent_b_id, operator_used,
                        code_hash, code_zstd, execution_status, raw_output,
                        primary_score, delta_score, novelty_score, composite_utility,
                        execution_time_ms, children_count, created_at
                 FROM experience_cards
                 WHERE task_id = ?1 AND code_hash = ?2
                 ORDER BY created_at DESC LIMIT 1",
                params![task_id, code_hash],
                Self::decode_card_row,
            )
            .optional()
            .map_err(StoreError::from)
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
        if self.get_task(input.task_id)?.is_none() {
            return Err(StoreError::NotFound(format!("task {}", input.task_id)));
        }

        let parent_a_score = match input.parent_a_id {
            Some(pid) => self
                .get_card(pid)?
                .ok_or_else(|| StoreError::NotFound(format!("parent_a card {pid}")))?
                .primary_score
                .into(),
            None => None,
        };
        if let Some(pid_b) = input.parent_b_id {
            if self.get_card(pid_b)?.is_none() {
                return Err(StoreError::NotFound(format!("parent_b card {pid_b}")));
            }
        }

        let raw_delta = match parent_a_score {
            Some(ps) => input.primary_score - ps,
            None => 0.0,
        };

        let bounds = self.get_task_score_bounds(input.task_id)?;
        let (min_s, max_s, min_d, max_d) = if bounds.total_cards == 0 {
            (
                0.0_f64.min(input.primary_score),
                input.primary_score.max(1.0),
                0.0_f64.min(raw_delta),
                raw_delta.max(1.0),
            )
        } else {
            (
                bounds.min_score.min(input.primary_score),
                bounds.max_score.max(input.primary_score),
                bounds.min_delta.min(raw_delta),
                bounds.max_delta.max(raw_delta),
            )
        };

        let metrics = CandidateMetrics {
            primary_score: input.primary_score,
            parent_primary_score: parent_a_score,
            children_count: 0,
        };
        let util = UtilityCalculator::calculate(&metrics, min_s, max_s, min_d, max_d, params);

        let code_hash = hash_code(input.code);
        let code_zstd = compress_code(input.code).map_err(StoreError::Validation)?;

        self.conn.execute(
            "INSERT INTO experience_cards (
                card_id, task_id, parent_a_id, parent_b_id, operator_used,
                code_hash, code_zstd, execution_status, raw_output,
                primary_score, delta_score, novelty_score, composite_utility,
                execution_time_ms, children_count
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 0)",
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
                util.raw_delta,
                util.novelty_score,
                util.composite,
                input.execution_time_ms,
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
        self.refresh_task_frontier(input.task_id, 10)?;

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

    /// Recompute `novelty_score` and `composite_utility` across all cards in a task
    /// whenever task score bounds or `children_count` shift.
    pub fn recompute_task_utilities(
        &self,
        task_id: &str,
        params: &UtilityHyperparams,
    ) -> StoreResult<()> {
        let bounds = self.get_task_score_bounds(task_id)?;
        if bounds.total_cards == 0 {
            return Ok(());
        }
        let cards = self.list_cards(task_id, None, None, 10_000)?;
        for card in cards {
            let parent_score = if card.parent_a_id.is_some() {
                Some(card.primary_score - card.delta_score)
            } else {
                None
            };
            let metrics = CandidateMetrics {
                primary_score: card.primary_score,
                parent_primary_score: parent_score,
                children_count: card.children_count,
            };
            let util = UtilityCalculator::calculate(
                &metrics,
                bounds.min_score,
                bounds.max_score,
                bounds.min_delta,
                bounds.max_delta,
                params,
            );
            self.conn.execute(
                "UPDATE experience_cards
                 SET novelty_score = ?1, composite_utility = ?2
                 WHERE card_id = ?3",
                params![util.novelty_score, util.composite, card.card_id],
            )?;
        }
        Ok(())
    }

    pub fn refresh_task_frontier(&self, task_id: &str, limit: usize) -> StoreResult<()> {
        self.conn.execute(
            "DELETE FROM task_frontiers WHERE task_id = ?1",
            params![task_id],
        )?;
        let mut stmt = self.conn.prepare(
            "SELECT card_id FROM experience_cards
             WHERE task_id = ?1 AND execution_status = 'SUCCESS'
             ORDER BY primary_score DESC, composite_utility DESC, execution_time_ms ASC
             LIMIT ?2",
        )?;
        let limit_i64 = i64::try_from(limit).unwrap_or(10);
        let ids: Vec<String> = stmt
            .query_map(params![task_id, limit_i64], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for (idx, card_id) in ids.iter().enumerate() {
            let rank = i64::try_from(idx.saturating_add(1)).unwrap_or(1);
            self.conn.execute(
                "INSERT INTO task_frontiers (task_id, card_id, rank) VALUES (?1, ?2, ?3)",
                params![task_id, card_id, rank],
            )?;
        }
        Ok(())
    }

    pub fn get_frontier(&self, task_id: &str) -> StoreResult<Vec<FrontierEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.card_id, c.task_id, c.parent_a_id, c.parent_b_id, c.operator_used,
                    c.code_hash, c.code_zstd, c.execution_status, c.raw_output,
                    c.primary_score, c.delta_score, c.novelty_score, c.composite_utility,
                    c.execution_time_ms, c.children_count, c.created_at, f.rank
             FROM task_frontiers f
             JOIN experience_cards c ON c.card_id = f.card_id
             WHERE f.task_id = ?1
             ORDER BY f.rank ASC",
        )?;
        let rows = stmt.query_map(params![task_id], |r| {
            let card = Self::decode_card_row(r)?;
            let rank: i64 = r.get(16)?;
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
        let mut stmt = self.conn.prepare(
            "SELECT card_id, task_id, parent_a_id, parent_b_id, operator_used,
                    code_hash, code_zstd, execution_status, raw_output,
                    primary_score, delta_score, novelty_score, composite_utility,
                    execution_time_ms, children_count, created_at
             FROM experience_cards
             WHERE task_id = ?1
               AND (?2 IS NULL OR operator_used = ?2)
               AND (?3 IS NULL OR execution_status = ?3)
             ORDER BY composite_utility DESC, created_at DESC
             LIMIT ?4",
        )?;
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
    /// For `DEBUG`, faulty candidates (`execution_status != 'SUCCESS'`) with the
    /// highest novelty score are preferred first; if none exist, falls back to
    /// composite-utility roulette selection across all candidates.
    pub fn sample_parent_by_utility(
        &self,
        task_id: &str,
        prefer_faulty: bool,
        exclude_card_id: Option<&str>,
    ) -> StoreResult<Option<ExperienceCard>> {
        let all_cards = self.list_cards(task_id, None, None, 500)?;
        if all_cards.is_empty() {
            return Ok(None);
        }

        let filtered: Vec<ExperienceCard> = all_cards
            .iter()
            .filter(|c| Some(c.card_id.as_str()) != exclude_card_id)
            .cloned()
            .collect();
        let pool_base = if filtered.is_empty() {
            all_cards
        } else {
            filtered
        };

        let candidates: Vec<ExperienceCard> = if prefer_faulty {
            let faulty: Vec<ExperienceCard> = pool_base
                .iter()
                .filter(|c| c.execution_status != "SUCCESS")
                .cloned()
                .collect();
            if faulty.is_empty() {
                pool_base
            } else {
                faulty
            }
        } else {
            let working: Vec<ExperienceCard> = pool_base
                .iter()
                .filter(|c| c.execution_status == "SUCCESS")
                .cloned()
                .collect();
            if working.is_empty() {
                pool_base
            } else {
                working
            }
        };

        if candidates.is_empty() {
            return Ok(None);
        }

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
        for (card, w) in candidates.iter().zip(weights.iter()) {
            accum += *w;
            if accum >= target {
                return Ok(Some(card.clone()));
            }
        }

        Ok(candidates.first().cloned())
    }

    /// Delete a task and all its associated frontier entries and experience cards.
    pub fn delete_task(&self, task_id: &str) -> StoreResult<bool> {
        if self.get_task(task_id)?.is_none() {
            return Ok(false);
        }
        self.conn.execute(
            "DELETE FROM task_frontiers WHERE task_id = ?1",
            params![task_id],
        )?;
        self.conn.execute(
            "UPDATE experience_cards SET parent_a_id = NULL, parent_b_id = NULL WHERE task_id = ?1",
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
    }

    /// Prune non-frontier experience cards for a task, keeping at least `keep_frontier` top cards.
    pub fn prune_cards(&self, task_id: &str, keep_frontier: usize) -> StoreResult<usize> {
        self.refresh_task_frontier(task_id, keep_frontier)?;
        let mut stmt = self.conn.prepare(
            "SELECT card_id FROM experience_cards
             WHERE task_id = ?1
               AND card_id NOT IN (SELECT card_id FROM task_frontiers WHERE task_id = ?1)
             ORDER BY composite_utility ASC",
        )?;
        let prune_ids: Vec<String> = stmt
            .query_map(params![task_id], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        if prune_ids.is_empty() {
            return Ok(0);
        }

        for id in &prune_ids {
            self.conn.execute(
                "UPDATE experience_cards SET parent_a_id = NULL WHERE parent_a_id = ?1",
                params![id],
            )?;
            self.conn.execute(
                "UPDATE experience_cards SET parent_b_id = NULL WHERE parent_b_id = ?1",
                params![id],
            )?;
            self.conn.execute(
                "DELETE FROM experience_cards WHERE card_id = ?1",
                params![id],
            )?;
        }
        Ok(prune_ids.len())
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
    pub fn import_task_bundle(&self, bundle_val: &Value) -> StoreResult<usize> {
        let bundle: TaskBundle = serde_json::from_value(bundle_val.clone())
            .map_err(|e| StoreError::Validation(format!("invalid task bundle JSON: {e}")))?;

        self.upsert_task(
            &bundle.task.task_id,
            &bundle.task.name,
            &bundle.task.description,
            &bundle.task.evaluator_script,
        )?;

        self.conn.execute("PRAGMA foreign_keys = OFF", [])?;

        let mut inserted_count = 0;
        for c in &bundle.cards {
            let code_hash = hash_code(&c.code);
            let code_zstd = compress_code(&c.code).map_err(StoreError::Validation)?;
            let rows = self.conn.execute(
                "INSERT INTO experience_cards (
                    card_id, task_id, parent_a_id, parent_b_id, operator_used,
                    code_hash, code_zstd, execution_status, raw_output,
                    primary_score, delta_score, novelty_score, composite_utility,
                    execution_time_ms, children_count, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                 ON CONFLICT(card_id) DO UPDATE SET
                    composite_utility = excluded.composite_utility,
                    primary_score = excluded.primary_score,
                    delta_score = excluded.delta_score,
                    novelty_score = excluded.novelty_score,
                    children_count = excluded.children_count",
                params![
                    c.card_id,
                    c.task_id,
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
                    c.created_at,
                ],
            )?;
            if rows > 0 {
                inserted_count += 1;
            }
        }

        for f in &bundle.frontier {
            let _ = self.conn.execute(
                "INSERT OR REPLACE INTO task_frontiers (task_id, card_id, rank) VALUES (?1, ?2, ?3)",
                params![bundle.task.task_id, f.card.card_id, f.rank],
            );
        }

        self.conn.execute("PRAGMA foreign_keys = ON", [])?;

        Ok(inserted_count)
    }
}
