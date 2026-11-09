-- Database Schema for RSICraft Stigmergic State Engine
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS tasks (
    task_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    evaluator_script TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS experience_cards (
    card_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    parent_a_id TEXT,
    parent_b_id TEXT,
    operator_used TEXT NOT NULL CHECK(operator_used IN ('DRAFT', 'IMPROVE', 'DEBUG', 'CROSSOVER')),
    code_hash TEXT NOT NULL,
    code_zstd BLOB NOT NULL,
    execution_status TEXT NOT NULL CHECK(execution_status IN ('SUCCESS', 'COMPILE_ERROR', 'RUNTIME_ERROR', 'TIMEOUT', 'METRIC_PARSE_ERROR')),
    raw_output TEXT NOT NULL,
    primary_score REAL NOT NULL DEFAULT 0.0,
    delta_score REAL NOT NULL DEFAULT 0.0,
    novelty_score REAL NOT NULL DEFAULT 1.0,
    composite_utility REAL NOT NULL DEFAULT 0.0,
    execution_time_ms INTEGER NOT NULL DEFAULT 0,
    children_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id),
    FOREIGN KEY(parent_a_id) REFERENCES experience_cards(card_id),
    FOREIGN KEY(parent_b_id) REFERENCES experience_cards(card_id)
);

CREATE TABLE IF NOT EXISTS task_frontiers (
    task_id TEXT NOT NULL,
    card_id TEXT NOT NULL,
    rank INTEGER NOT NULL,
    PRIMARY KEY (task_id, card_id),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id),
    FOREIGN KEY(card_id) REFERENCES experience_cards(card_id)
);

CREATE INDEX IF NOT EXISTS idx_cards_task_utility ON experience_cards(task_id, composite_utility DESC);
CREATE INDEX IF NOT EXISTS idx_cards_hash ON experience_cards(code_hash);
CREATE INDEX IF NOT EXISTS idx_cards_parents ON experience_cards(parent_a_id, parent_b_id);
