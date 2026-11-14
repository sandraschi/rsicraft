//! `rsicraft-app`: L3 Pure-Rust `eframe` / `egui 0.37` GPU Desktop Studio.
//!
//! Adheres strictly to `EGUI_DESKTOP_UI_AND_PARITY.md`:
//! - High-contrast `slate-950` (`#0a0d12`) dark theme + crisp `slate-50` light theme
//! - Vector-painted Sun / Crescent Moon toggle and `Ctrl + MouseWheel` zoom
//! - Zero non-ASCII tofu glyphs (verified by `glyph_allowlist_no_tofu_chars` test)
//! - Interactive GPU-painted Bezier Lineage DAG Canvas and Score Trajectory Spark Chart
//! - Full 4-Operator Evolutionary Launchpad, BLAKE3+Zstd CAS Inspector, RTX 4090 Gauge,
//!   "What is RSI?" page, and the searchable 10-tab help (same source as the CLI and MCP help).
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Pos2, RichText, Stroke, Vec2};
use rsicraft_core::llm::failure_kind_of;
use rsicraft_core::operators::SearchOperator;
use rsicraft_core::sandbox::{SandboxExecutionResult, SandboxRunner};
use rsicraft_core::{
    campaign_operator, commit_evaluate, commit_step, dispatch, plan_evaluate, plan_step,
    run_evaluate, run_step, tools, Command, Ctx, EvalPlan, Registry, StepOutcome, StepPlan,
    CAMPAIGN_MAX_CONSECUTIVE_FAILURES, VERSION,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::{Duration, Instant};

const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
const ACCENT_EMERALD: Color32 = Color32::from_rgb(52, 211, 153);
const ACCENT_AMBER: Color32 = Color32::from_rgb(251, 191, 36);
const ACCENT_PURPLE: Color32 = Color32::from_rgb(192, 132, 252);
const ACCENT_ROSE: Color32 = Color32::from_rgb(251, 113, 133);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioPage {
    EvolutionaryStudio,
    LineageDagCanvas,
    ExperienceCards,
    TaskAndSandboxLab,
    Goliath4090AndLlm,
    RsiAndTsinghuaDossier,
    MultitabHelpAndTools,
}

impl StudioPage {
    #[must_use]
    pub const fn all() -> [Self; 7] {
        [
            Self::EvolutionaryStudio,
            Self::LineageDagCanvas,
            Self::ExperienceCards,
            Self::TaskAndSandboxLab,
            Self::Goliath4090AndLlm,
            Self::RsiAndTsinghuaDossier,
            Self::MultitabHelpAndTools,
        ]
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::EvolutionaryStudio => "1. Evolutionary Studio",
            Self::LineageDagCanvas => "2. Lineage DAG Canvas",
            Self::ExperienceCards => "3. Experience Cards & CAS",
            Self::TaskAndSandboxLab => "4. Task & Sandbox Lab",
            Self::Goliath4090AndLlm => "5. RTX 4090 & LLM Config",
            Self::RsiAndTsinghuaDossier => "6. What is RSI?",
            Self::MultitabHelpAndTools => "7. Help",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Cards,
    Table,
    Json,
}

/// The five Help dialogs from the Craft menu-bar standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioDialog {
    CommandPalette,
    Shortcuts,
    Docs,
    Licenses,
    About,
}

/// Menu and command-palette entries. `id` is what `menu_action` accepts.
const MENU_CATALOG: &[(&str, &str, &str)] = &[
    ("new_task", "New Task...", "Ctrl+N"),
    ("open_db", "Open Database...", "Ctrl+O"),
    ("save", "Save", "Ctrl+S"),
    ("save_as", "Save As...", "Ctrl+Shift+S"),
    ("export", "Export Lineage...", "Ctrl+E"),
    ("close_task", "Close Task", "Ctrl+W"),
    ("quit", "Quit", "Ctrl+Q"),
    ("undo", "Undo", "Ctrl+Z"),
    ("redo", "Redo", "Ctrl+Y"),
    ("cut", "Cut", "Ctrl+X"),
    ("copy_card", "Copy", "Ctrl+C"),
    ("paste", "Paste", "Ctrl+V"),
    ("select_all", "Select All", "Ctrl+A"),
    ("preferences", "Preferences...", "Ctrl+,"),
    ("view_lineage", "Lineage DAG Canvas", ""),
    ("view_cards", "Experience Cards", ""),
    ("view_lab", "Task and Sandbox Lab", ""),
    ("view_studio", "Evolutionary Studio", ""),
    ("view_gpu", "RTX 4090 and LLM", ""),
    ("view_help", "Help and Tools", ""),
    ("theme_dark", "Theme Dark", ""),
    ("theme_light", "Theme Light", ""),
    ("zoom_reset", "Reset Zoom (100%)", "Ctrl+1"),
    ("seed", "Add Seed Candidate", ""),
    ("insert_spec", "Insert Problem Specification", ""),
    ("step_improve", "Mutate Candidate (IMPROVE)", ""),
    ("step_crossover", "Cross Parents (CROSSOVER)", ""),
    ("step_debug", "Repair Candidate (DEBUG)", ""),
    ("prune", "Prune Frontier", ""),
    ("hyperparams", "Tune Hyperparameters", ""),
    ("campaign", "Run Evolutionary Loop", ""),
    ("probe", "Probe LLM Endpoint", ""),
    ("frontier", "Calculate Pareto Frontier", ""),
    ("reset_layout", "Reset Panel Layout", ""),
    ("palette", "Command Palette...", "Ctrl+K"),
    ("shortcuts", "Keyboard Shortcuts...", "F1"),
    ("docs", "Project Files and Docs...", ""),
    ("licenses", "Licenses and Notices...", ""),
    ("about", "About RSICraft", ""),
    ("status", "Store Status", ""),
    ("hardware", "Hardware Profile", ""),
    ("lineage", "Refresh Lineage", ""),
    ("tasks", "List Tasks", ""),
    ("help_api", "Operator Manual", ""),
];

const UNDO_CAP: usize = 8;

struct StoreSnap {
    label: &'static str,
    path: Option<PathBuf>,
    bytes: Vec<u8>,
}

pub struct RsiCraftApp {
    pub ctx: Ctx,
    pub page: StudioPage,
    pub view_mode: ViewMode,
    pub dark_mode: bool,
    pub style_initialized: bool,
    pub anim_tick: u64,

    // Active task & evolutionary controls
    pub active_task_id: String,
    pub provider_pick: String,
    pub sandbox_pick: String,
    pub campaign_steps: u32,
    pub timeout_secs: u64,
    pub llama_url_edit: String,
    pub model_alias_edit: String,

    // Cached JSON payloads
    pub status_json: Value,
    pub hw_json: Value,
    pub frontier_json: Value,
    pub cards_json: Value,
    pub lineage_json: Value,
    pub tasks_json: Value,
    pub selected_card_json: Option<Value>,
    pub help_json: Value,

    // Filters & selections
    pub card_search: String,
    pub op_filter: String,
    pub status_filter: String,
    pub selected_help_tab: String,
    pub help_search: String,
    pub show_quick_help_modal: bool,

    // Task & Sandbox Lab editors
    pub edit_task_id: String,
    pub edit_task_name: String,
    pub edit_task_desc: String,
    pub edit_evaluator: String,
    pub lab_candidate_code: String,
    pub lab_last_eval: Option<Value>,
    pub card_diff_output: Option<String>,

    pub status_banner: String,
    pub repo_root: PathBuf,
    pub active_dialog: Option<StudioDialog>,
    pub palette_query: String,
    pub theme_dirty: bool,
    pub quit_requested: bool,
    pub pending_copy: Option<String>,
    pub zoom_reset: bool,
    undo: Vec<StoreSnap>,
    redo: Vec<StoreSnap>,
    open_recent: Vec<PathBuf>,
    pick_open: bool,
    pick_save: bool,
    pending_recent: Option<usize>,

    // 3D RTX 4090 GPU Studio State
    pub gpu_rot_x: f32,
    pub gpu_rot_y: f32,
    pub gpu_auto_spin: bool,
    pub gpu_spin_speed: f32,
    pub gpu_zoom: f32,
    pub gpu_style: usize,
    pub gpu_show_vram_chips: bool,
    pub gpu_plasma_fx: bool,

    // 3D RSI Evolutionary Landscape & Manifold State
    pub dag_3d_mode: bool,
    pub rsi_3d_rot_x: f32,
    pub rsi_3d_rot_y: f32,
    pub rsi_3d_auto_spin: bool,
    pub rsi_3d_zoom: f32,

    // Background run state: LLM and sandbox work happens on a worker thread.
    pub job: Option<ActiveJob>,
    pub run_feed: Vec<RunEvent>,
    pub repaint_ctx: Option<egui::Context>,

    /// Last result of "Check provider" (the `rsi_llm_probe` report).
    pub provider_report: Option<Value>,
    /// Last step failure: (class, full message with fix). Shown until the next
    /// successful step so a failed run is never silent.
    pub last_failure: Option<(String, String)>,
    /// A provider check running in the background.
    provider_check: Option<Receiver<Value>>,
}

/// One finished step as shown in the live run timeline.
#[derive(Debug, Clone)]
pub struct RunEvent {
    pub operator: String,
    pub status: String,
    pub score: f64,
    pub delta: f64,
    pub secs: f32,
    pub card_id: String,
    pub new_best: bool,
}

/// Result handed back from the worker thread. The plan travels with it so the
/// UI thread can commit exactly what was planned.
enum WorkerMsg {
    Step(Box<StepPlan>, Result<StepOutcome, String>),
    Eval(Box<EvalPlan>, Result<SandboxExecutionResult, String>),
}

struct CampaignSpec {
    task_id: String,
    provider: String,
    sandbox_mode: String,
    timeout_secs: u64,
}

/// A run in progress. The slow phase (LLM inference and sandbox execution)
/// runs on a worker thread; the SQLite store is only touched on the UI thread.
pub struct ActiveJob {
    rx: Receiver<WorkerMsg>,
    pub label: String,
    pub current: String,
    pub done: u32,
    pub total: u32,
    pub started: Instant,
    pub step_started: Instant,
    pub stop_requested: bool,
    /// Steps in a row that produced no card.
    pub consecutive_failures: u32,
    campaign: Option<CampaignSpec>,
}

fn default_repo_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("docs").join("brochure").exists() {
        cwd
    } else {
        PathBuf::from("D:\\Dev\\repos\\rsicraft")
    }
}

impl RsiCraftApp {
    pub fn new(db_path: Option<&Path>) -> Result<Self, String> {
        let reg = match db_path {
            Some(p) => Registry::open(p).map_err(|e| e.to_string())?,
            None => Registry::open_in_memory().map_err(|e| e.to_string())?,
        };
        let mut ctx = Ctx::new(reg, None, None, None);
        // Seed demo data automatically if the store is empty so the studio opens with a rich 4-card DAG
        let st = ctx.reg.status().map_err(|e| e.to_string())?;
        if st.tasks_count == 0 && st.cards_count == 0 {
            let _ = dispatch(&mut ctx, Command::DemoSeed);
        }

        let llm_url_init = ctx.llm.base_url.clone();
        let llm_model_init = ctx.llm.model_alias.clone();
        let mut app = Self {
            ctx,
            page: StudioPage::EvolutionaryStudio,
            view_mode: ViewMode::Cards,
            dark_mode: true,
            style_initialized: false,
            anim_tick: 0,
            active_task_id: "opt-synth".to_string(),
            provider_pick: "demo".to_string(),
            sandbox_pick: "demo".to_string(),
            campaign_steps: 4,
            timeout_secs: 30,
            llama_url_edit: llm_url_init,
            model_alias_edit: llm_model_init,
            provider_report: None,
            last_failure: None,
            provider_check: None,
            status_json: json!({}),
            hw_json: json!({}),
            frontier_json: json!({}),
            cards_json: json!({}),
            lineage_json: json!({}),
            tasks_json: json!({}),
            selected_card_json: None,
            help_json: json!({}),
            card_search: String::new(),
            op_filter: "ALL".to_string(),
            status_filter: "ALL".to_string(),
            selected_help_tab: "all".to_string(),
            help_search: String::new(),
            show_quick_help_modal: false,
            edit_task_id: "kadane-4090".to_string(),
            edit_task_name: "Maximum Contiguous Subarray (RTX 4090)".to_string(),
            edit_task_desc:
                "Write solve(arr) returning the maximum contiguous subarray sum in O(N) time."
                    .to_string(),
            edit_evaluator: "import json, candidate\ns = float(candidate.solve([-2, 1, -3, 4, -1, 2, 1, -5, 4]))\nprint(f'RSI_METRIC_JSON={json.dumps({\"status\": \"SUCCESS\", \"score\": s * 16.0})}')".to_string(),
            lab_candidate_code: "def solve(arr):\n    if not arr:\n        return 0.0\n    best = cur = arr[0]\n    for x in arr[1:]:\n        cur = cur + x if cur > 0 else x\n        if cur > best:\n            best = cur\n    return float(best)\n".to_string(),
            lab_last_eval: None,
            card_diff_output: None,
            status_banner: format!(
                "rsicraft v{VERSION} ready. New here? Press [? New here] or open the Help page."
            ),
            repo_root: default_repo_root(),
            active_dialog: None,
            palette_query: String::new(),
            theme_dirty: false,
            quit_requested: false,
            pending_copy: None,
            zoom_reset: false,
            undo: Vec::new(),
            redo: Vec::new(),
            open_recent: Vec::new(),
            pick_open: false,
            pick_save: false,
            pending_recent: None,
            gpu_rot_x: -0.32,
            gpu_rot_y: 0.65,
            gpu_auto_spin: true,
            gpu_spin_speed: 0.012,
            gpu_zoom: 1.0,
            gpu_style: 0,
            gpu_show_vram_chips: true,
            gpu_plasma_fx: true,
            dag_3d_mode: false,
            rsi_3d_rot_x: -0.38,
            rsi_3d_rot_y: 0.52,
            rsi_3d_auto_spin: true,
            rsi_3d_zoom: 1.0,
            job: None,
            run_feed: Vec::new(),
            repaint_ctx: None,
        };
        app.refresh_all();
        Ok(app)
    }

    pub fn exec(&mut self, cmd: Command) -> Result<Value, String> {
        dispatch(&mut self.ctx, cmd)
    }

    fn remember_before(&mut self, label: &'static str) -> bool {
        match self.ctx.reg.snapshot_bytes() {
            Ok(bytes) => {
                self.redo.clear();
                self.undo.push(StoreSnap {
                    label,
                    path: self.ctx.reg.db_path().map(Path::to_path_buf),
                    bytes,
                });
                if self.undo.len() > UNDO_CAP {
                    let extra = self.undo.len() - UNDO_CAP;
                    self.undo.drain(..extra);
                }
                true
            }
            Err(e) => {
                self.status_banner = format!("Could not snapshot the store: {e}");
                false
            }
        }
    }

    fn adopt_store(&mut self) {
        self.ctx.hyperparams = self.ctx.reg.load_hyperparams();
        self.refresh_all();
    }

    /// Run a store mutation that Undo can put back.
    pub fn undoable(&mut self, label: &'static str, cmd: Command) -> Result<Value, String> {
        if self.job.is_some() {
            let msg = "A run is in progress. Undoable edits wait until it finishes.".to_string();
            self.status_banner = msg.clone();
            return Err(msg);
        }
        if !self.remember_before(label) {
            return Err(self.status_banner.clone());
        }
        match self.exec(cmd) {
            Ok(v) => Ok(v),
            Err(e) => {
                self.undo.pop();
                Err(e)
            }
        }
    }

    fn restore_snap(&mut self, snap: StoreSnap) -> Result<(), String> {
        self.ctx
            .reg
            .restore_bytes(&snap.bytes, snap.path)
            .map_err(|e| e.to_string())?;
        self.adopt_store();
        Ok(())
    }

    fn undo_store(&mut self) -> Result<Value, String> {
        if self.job.is_some() {
            self.status_banner = "Undo waits until the run finishes".into();
            return Ok(json!({ "undo": "busy" }));
        }
        let Some(snap) = self.undo.pop() else {
            self.status_banner = "Nothing to undo".into();
            return Ok(json!({ "undo": false }));
        };
        let label = snap.label;
        let current = match self.ctx.reg.snapshot_bytes() {
            Ok(bytes) => StoreSnap {
                label: "redo",
                path: self.ctx.reg.db_path().map(Path::to_path_buf),
                bytes,
            },
            Err(e) => {
                self.undo.push(snap);
                return Err(e.to_string());
            }
        };
        self.restore_snap(snap)?;
        self.redo.push(current);
        self.status_banner = format!("Undid {label}");
        Ok(json!({ "undo": label }))
    }

    fn redo_store(&mut self) -> Result<Value, String> {
        if self.job.is_some() {
            self.status_banner = "Redo waits until the run finishes".into();
            return Ok(json!({ "redo": "busy" }));
        }
        let Some(snap) = self.redo.pop() else {
            self.status_banner = "Nothing to redo".into();
            return Ok(json!({ "redo": false }));
        };
        let current = match self.ctx.reg.snapshot_bytes() {
            Ok(bytes) => StoreSnap {
                label: "undo",
                path: self.ctx.reg.db_path().map(Path::to_path_buf),
                bytes,
            },
            Err(e) => {
                self.redo.push(snap);
                return Err(e.to_string());
            }
        };
        self.restore_snap(snap)?;
        self.undo.push(current);
        self.status_banner = "Redid the last edit".into();
        Ok(json!({ "redo": true }))
    }

    fn note_recent(&mut self, path: PathBuf) {
        self.open_recent.retain(|p| p != &path);
        self.open_recent.insert(0, path);
        self.open_recent.truncate(5);
    }

    pub fn save_store_as(&mut self, path: PathBuf) -> Result<(), String> {
        if self.job.is_some() {
            return Err("A run is in progress".into());
        }
        let bytes = self.ctx.reg.snapshot_bytes().map_err(|e| e.to_string())?;
        if !self.remember_before("save as") {
            return Err(self.status_banner.clone());
        }
        self.ctx
            .reg
            .restore_bytes(&bytes, Some(path.clone()))
            .map_err(|e| e.to_string())?;
        self.adopt_store();
        self.note_recent(path.clone());
        self.status_banner = format!("Saved {}", path.display());
        Ok(())
    }

    pub fn open_store_at(&mut self, path: PathBuf) -> Result<(), String> {
        if self.job.is_some() {
            return Err("A run is in progress".into());
        }
        if !path.is_file() {
            return Err(format!("No database at {}", path.display()));
        }
        if !self.remember_before("open database") {
            return Err(self.status_banner.clone());
        }
        match Registry::open(&path) {
            Ok(reg) => {
                self.ctx.reg = reg;
                self.adopt_store();
                self.note_recent(path.clone());
                self.status_banner = format!("Opened {}", path.display());
                Ok(())
            }
            Err(e) => {
                self.undo.pop();
                Err(e.to_string())
            }
        }
    }

    fn browse_open(&mut self) {
        let mut dialog = rfd::FileDialog::new().add_filter("SQLite", &["db", "sqlite"]);
        if let Some(path) = self.ctx.reg.db_path().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(path);
        }
        if let Some(path) = dialog.pick_file() {
            if let Err(e) = self.open_store_at(path) {
                self.status_banner = e;
            }
        }
    }

    fn browse_save(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("SQLite", &["db"])
            .set_file_name("rsicraft.db");
        if let Some(path) = self.ctx.reg.db_path().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(path);
        }
        if let Some(mut path) = dialog.save_file() {
            if path.extension().is_none() {
                path.set_extension("db");
            }
            if let Err(e) = self.save_store_as(path) {
                self.status_banner = e;
            }
        }
    }

    fn apply_file_picks(&mut self) {
        if self.job.is_some() {
            if self.pick_open || self.pick_save || self.pending_recent.is_some() {
                self.status_banner = "File changes wait until the run finishes".into();
            }
            self.pick_open = false;
            self.pick_save = false;
            self.pending_recent = None;
            return;
        }
        if self.pick_open {
            self.pick_open = false;
            self.browse_open();
        }
        if self.pick_save {
            self.pick_save = false;
            self.browse_save();
        }
        if let Some(i) = self.pending_recent.take() {
            if let Some(path) = self.open_recent.get(i).cloned() {
                if let Err(e) = self.open_store_at(path) {
                    self.status_banner = e;
                }
            }
        }
    }

    pub fn refresh_all(&mut self) {
        if let Ok(v) = self.exec(Command::Status) {
            self.status_json = v;
        }
        if let Ok(v) = self.exec(Command::HardwareProfile) {
            self.hw_json = v;
        }
        if let Ok(v) = self.exec(Command::TaskList) {
            self.tasks_json = v;
        }
        let tid = self.active_task_id.clone();
        if let Ok(v) = self.exec(Command::GetFrontier {
            task_id: tid.clone(),
            limit: 10,
        }) {
            self.frontier_json = v;
        }
        let op_f = if self.op_filter == "ALL" {
            None
        } else {
            Some(self.op_filter.clone())
        };
        let st_f = if self.status_filter == "ALL" {
            None
        } else {
            Some(self.status_filter.clone())
        };
        if let Ok(v) = self.exec(Command::CardList {
            task_id: tid.clone(),
            operator: op_f,
            status: st_f,
            limit: 100,
        }) {
            self.cards_json = v;
        }
        if let Ok(v) = self.exec(Command::Lineage { task_id: tid }) {
            self.lineage_json = v;
        }
        if self.selected_card_json.is_none() {
            if let Some(first_id) = self
                .frontier_json
                .get("frontier")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .and_then(|c| c.get("card_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
            {
                self.selected_card_json = self.exec(Command::CardShow { card_id: first_id }).ok();
            }
        }
        self.refresh_help();
    }

    /// Point the engine at the server address and model typed on the LLM page.
    pub fn apply_endpoint(&mut self) {
        self.ctx.llm = rsicraft_core::llm::LlamaClient::new(
            self.llama_url_edit.trim(),
            self.model_alias_edit.trim(),
        );
        self.llama_url_edit.clone_from(&self.ctx.llm.base_url);
        self.status_banner = format!(
            "Model server set to {} (model {})",
            self.ctx.llm.base_url, self.ctx.llm.model_alias
        );
        self.refresh_all();
    }

    /// Check the configured server and model in the background; the report
    /// appears on the LLM page and a summary in the status line.
    pub fn check_provider(&mut self) {
        let client = self.ctx.llm.clone();
        let (tx, rx) = channel();
        let repaint = self.repaint_ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(client.diagnose());
            if let Some(c) = repaint {
                c.request_repaint();
            }
        });
        self.provider_check = Some(rx);
        self.status_banner = format!("Checking {} ...", self.ctx.llm.base_url);
    }

    fn poll_provider_check(&mut self) {
        let Some(rx) = &self.provider_check else {
            return;
        };
        let report = match rx.try_recv() {
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => json!({
                "ok": false,
                "failure": {"kind": "config", "message": "the provider check ended without a result", "remedy": "try again"}
            }),
            Ok(v) => v,
        };
        self.provider_check = None;
        self.status_banner = if report.get("ok").and_then(Value::as_bool) == Some(true) {
            format!(
                "Provider OK: {} serves {}",
                report
                    .get("base_url")
                    .and_then(Value::as_str)
                    .unwrap_or("?"),
                report
                    .get("configured_model")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
            )
        } else {
            let kind = report
                .pointer("/failure/kind")
                .and_then(Value::as_str)
                .unwrap_or("error");
            format!("Provider NOT usable ({kind}); see the LLM page for the fix")
        };
        self.provider_report = Some(report);
    }

    pub fn refresh_help(&mut self) {
        let tab = Some(self.selected_help_tab.clone());
        let search = if self.help_search.trim().is_empty() {
            None
        } else {
            Some(self.help_search.trim().to_string())
        };
        if let Ok(v) = self.exec(Command::Help { tab, search }) {
            self.help_json = v;
        }
    }

    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    fn refuse_if_busy(&mut self) -> bool {
        if self.is_busy() {
            self.status_banner =
                "A run is already in progress; wait for it or stop the campaign".to_string();
            return true;
        }
        false
    }

    /// Plan a step on this thread (store read), then run the slow LLM and
    /// sandbox phase on a worker thread. `None` when the step could not start.
    fn spawn_step(
        &mut self,
        task_id: &str,
        op: SearchOperator,
        provider: &str,
        sandbox_mode: &str,
        timeout_secs: u64,
        preflight: bool,
    ) -> Option<Receiver<WorkerMsg>> {
        let plan = match plan_step(&self.ctx, task_id, op, provider, sandbox_mode, timeout_secs) {
            Ok(p) => p,
            Err(e) => {
                self.status_banner = format!("Step {} error: {e}", op.as_str());
                return None;
            }
        };
        let (tx, rx) = channel();
        let llm = self.ctx.llm.clone();
        let sandbox = SandboxRunner::new(self.ctx.sandbox.image_tag.clone());
        let repaint = self.repaint_ctx.clone();
        std::thread::spawn(move || {
            // First step of a run against a real model: check the server and
            // model first, so a setup problem stops the run with its fix
            // instead of failing step after step.
            let checked = if preflight && plan.provider == "llama" {
                let mut client = llm.clone();
                client.model_alias.clone_from(&plan.model_alias);
                client.preflight().map(|_| ()).map_err(String::from)
            } else {
                Ok(())
            };
            let res = checked.and_then(|()| run_step(&plan, &llm, &sandbox));
            let _ = tx.send(WorkerMsg::Step(Box::new(plan), res));
            if let Some(c) = repaint {
                c.request_repaint();
            }
        });
        Some(rx)
    }

    fn new_job(
        rx: Receiver<WorkerMsg>,
        label: String,
        current: &str,
        total: u32,
        campaign: Option<CampaignSpec>,
    ) -> ActiveJob {
        let now = Instant::now();
        ActiveJob {
            rx,
            label,
            current: current.to_string(),
            done: 0,
            total,
            started: now,
            step_started: now,
            stop_requested: false,
            consecutive_failures: 0,
            campaign,
        }
    }

    /// Start one evolutionary step in the background.
    pub fn trigger_step(&mut self, op: SearchOperator) {
        if self.refuse_if_busy() {
            return;
        }
        let tid = self.active_task_id.clone();
        let prov = self.provider_pick.clone();
        let sb = self.sandbox_pick.clone();
        if let Some(rx) = self.spawn_step(&tid, op, &prov, &sb, self.timeout_secs, true) {
            self.job = Some(Self::new_job(
                rx,
                format!("Step {}", op.as_str()),
                op.as_str(),
                1,
                None,
            ));
        }
    }

    /// Start a multi-step campaign in the background. Steps run one at a time;
    /// each is committed and shown before the next one starts.
    pub fn start_campaign(&mut self) {
        if self.refuse_if_busy() {
            return;
        }
        let spec = CampaignSpec {
            task_id: self.active_task_id.clone(),
            provider: self.provider_pick.clone(),
            sandbox_mode: self.sandbox_pick.clone(),
            timeout_secs: self.timeout_secs,
        };
        let total = self.campaign_steps.max(1);
        let op = match campaign_operator(&self.ctx, &spec.task_id, 0) {
            Ok(op) => op,
            Err(e) => {
                self.status_banner = format!("Campaign error: {e}");
                return;
            }
        };
        if let Some(rx) = self.spawn_step(
            &spec.task_id,
            op,
            &spec.provider,
            &spec.sandbox_mode,
            spec.timeout_secs,
            true,
        ) {
            self.job = Some(Self::new_job(
                rx,
                format!("{total}-step campaign"),
                op.as_str(),
                total,
                Some(spec),
            ));
        }
    }

    /// Evaluate the lab candidate in the background and record it as a card.
    pub fn start_evaluate(&mut self) {
        if self.refuse_if_busy() {
            return;
        }
        let plan = match plan_evaluate(
            &self.ctx,
            &self.active_task_id,
            &self.lab_candidate_code,
            SearchOperator::Draft,
            None,
            None,
            &self.sandbox_pick,
            self.timeout_secs,
            true,
        ) {
            Ok(p) => p,
            Err(e) => {
                self.status_banner = format!("Evaluation error: {e}");
                return;
            }
        };
        let (tx, rx) = channel();
        let sandbox = SandboxRunner::new(self.ctx.sandbox.image_tag.clone());
        let repaint = self.repaint_ctx.clone();
        std::thread::spawn(move || {
            let res = run_evaluate(&plan, &sandbox);
            let _ = tx.send(WorkerMsg::Eval(Box::new(plan), res));
            if let Some(c) = repaint {
                c.request_repaint();
            }
        });
        self.job = Some(Self::new_job(
            rx,
            "Sandbox evaluation".to_string(),
            "EVALUATE",
            1,
            None,
        ));
    }

    /// Ask a running campaign to end after the step that is in flight.
    pub fn request_stop(&mut self) {
        if let Some(job) = self.job.as_mut() {
            job.stop_requested = true;
        }
    }

    fn push_run_event(&mut self, step: &Value, secs: f32) {
        let card_id = step
            .get("card_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let best_id = self
            .frontier_json
            .get("frontier")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|c| c.get("card_id"))
            .and_then(Value::as_str);
        let str_of = |key: &str| {
            step.get(key)
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string()
        };
        let num_of = |key: &str| step.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        self.run_feed.push(RunEvent {
            operator: str_of("operator"),
            status: str_of("status"),
            score: num_of("primary_score"),
            delta: num_of("delta_score"),
            secs,
            new_best: !card_id.is_empty() && best_id == Some(card_id.as_str()),
            card_id,
        });
        if self.run_feed.len() > 200 {
            let excess = self.run_feed.len() - 200;
            self.run_feed.drain(..excess);
        }
    }

    /// Collect a finished worker result, commit it to the store on this
    /// thread, and start the next campaign step if there is one. Called once
    /// per frame; returns immediately when nothing has finished.
    pub fn poll_job(&mut self) {
        self.poll_provider_check();
        let msg = match self.job.as_ref().map(|j| j.rx.try_recv()) {
            None | Some(Err(TryRecvError::Empty)) => return,
            Some(Err(TryRecvError::Disconnected)) => {
                self.job = None;
                self.status_banner =
                    "Run failed: the worker thread ended without a result".to_string();
                return;
            }
            Some(Ok(m)) => m,
        };
        let Some(mut job) = self.job.take() else {
            return;
        };
        match msg {
            WorkerMsg::Eval(plan, res) => {
                let snapped = res.is_ok() && self.remember_before("evaluate");
                match res.and_then(|r| commit_evaluate(&self.ctx, &plan, &r)) {
                    Ok(v) => {
                        self.lab_last_eval = Some(v);
                        self.status_banner =
                            "Sandbox evaluation completed and recorded".to_string();
                        self.refresh_all();
                    }
                    Err(e) => {
                        if snapped {
                            self.undo.pop();
                        }
                        self.status_banner = format!("Evaluation error: {e}");
                    }
                }
            }
            WorkerMsg::Step(plan, res) => {
                let secs = job.step_started.elapsed().as_secs_f32();
                let snapped = res.is_ok() && self.remember_before("step");
                match res.and_then(|o| commit_step(&self.ctx, &plan, &o)) {
                    Ok(step) => {
                        job.done += 1;
                        job.consecutive_failures = 0;
                        self.last_failure = None;
                        self.refresh_all();
                        self.push_run_event(&step, secs);
                        let cid = step.get("card_id").and_then(Value::as_str).unwrap_or("?");
                        let score = step
                            .get("primary_score")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0);
                        self.status_banner = format!(
                            "{}: {} of {} done, {} -> card {} (score={score:.4})",
                            job.label,
                            job.done,
                            job.total,
                            plan.operator.as_str(),
                            &cid[..cid.len().min(8)]
                        );
                        if let Ok(card) = self.exec(Command::CardShow {
                            card_id: cid.to_string(),
                        }) {
                            self.selected_card_json = Some(card);
                        }
                    }
                    Err(e) => {
                        if snapped {
                            self.undo.pop();
                        }
                        // No card came out of this step. Setup problems (server
                        // unreachable, unknown model, missing key, bad address)
                        // end the run at once; otherwise a campaign skips one
                        // such step and carries on, and two in a row end it.
                        let kind = failure_kind_of(&e);
                        let class = kind.map_or("error", |k| k.as_str()).to_string();
                        self.last_failure = Some((class.clone(), e.clone()));
                        job.consecutive_failures += 1;
                        let fatal = kind.is_some_and(|k| k.is_fatal());
                        let give_up = fatal
                            || job.campaign.is_none()
                            || job.consecutive_failures >= CAMPAIGN_MAX_CONSECUTIVE_FAILURES;
                        self.run_feed.push(RunEvent {
                            operator: plan.operator.as_str().to_string(),
                            status: class.to_ascii_uppercase(),
                            score: 0.0,
                            delta: 0.0,
                            secs,
                            card_id: String::new(),
                            new_best: false,
                        });
                        if give_up {
                            self.status_banner = format!(
                                "{} stopped after {} of {} steps ({class}); see Live Run for the fix",
                                job.label, job.done, job.total
                            );
                            return;
                        }
                        job.done += 1;
                        self.status_banner = format!(
                            "{}: step {} skipped ({class}); see Live Run for the fix",
                            job.label, job.done
                        );
                    }
                }
                let more = job.done < job.total && !job.stop_requested;
                if let (true, Some(spec)) = (more, job.campaign.as_ref()) {
                    let next = match campaign_operator(&self.ctx, &spec.task_id, job.done) {
                        Ok(op) => op,
                        Err(e) => {
                            self.status_banner = format!("Campaign error: {e}");
                            return;
                        }
                    };
                    // spawn_step sets the banner itself when it cannot start.
                    if let Some(rx) = self.spawn_step(
                        &spec.task_id,
                        next,
                        &spec.provider,
                        &spec.sandbox_mode,
                        spec.timeout_secs,
                        false,
                    ) {
                        job.rx = rx;
                        job.current = next.as_str().to_string();
                        job.step_started = Instant::now();
                        self.job = Some(job);
                    }
                } else if job.campaign.is_some() {
                    self.status_banner = format!(
                        "{} {}: {} of {} steps in {:.0}s",
                        job.label,
                        if job.stop_requested && job.done < job.total {
                            "stopped"
                        } else {
                            "finished"
                        },
                        job.done,
                        job.total,
                        job.started.elapsed().as_secs_f32()
                    );
                }
            }
        }
    }

    /// Block until the current job has finished (headless use and tests).
    /// Returns false when `timeout` passed first.
    pub fn wait_for_job(&mut self, timeout: Duration) -> bool {
        let start = Instant::now();
        loop {
            self.poll_job();
            if self.job.is_none() {
                return true;
            }
            if start.elapsed() >= timeout {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Live view of the running job and a timeline of the steps finished in
    /// this session: operator, score bar scaled to the best score so far,
    /// delta against the parent, and wall time per step.
    fn render_live_run_panel(&mut self, ui: &mut egui::Ui) {
        if self.job.is_none() && self.run_feed.is_empty() {
            return;
        }
        ui.add_space(6.0);
        let mut stop_clicked = false;
        let mut clear_clicked = false;
        accent_card(ui, ACCENT_CYAN, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Live Run");
                if let Some(job) = &self.job {
                    ui.add(egui::Spinner::new());
                    ui.label(
                        RichText::new(&job.current)
                            .color(operator_color(&job.current))
                            .strong(),
                    );
                    ui.label(format!(
                        "{}: step {} of {} | this step {:.0}s | total {:.0}s",
                        job.label,
                        (job.done + 1).min(job.total),
                        job.total,
                        job.step_started.elapsed().as_secs_f32(),
                        job.started.elapsed().as_secs_f32()
                    ));
                    if job.campaign.is_some() {
                        if job.stop_requested {
                            ui.label("stopping after this step");
                        } else if ui.button("[Stop after this step]").clicked() {
                            stop_clicked = true;
                        }
                    }
                } else {
                    ui.label(format!("idle | {} steps this session", self.run_feed.len()));
                    if ui.small_button("Clear").clicked() {
                        clear_clicked = true;
                    }
                }
            });
            if let Some(job) = &self.job {
                let frac = job.done as f32 / job.total.max(1) as f32;
                ui.add(egui::ProgressBar::new(frac).desired_height(6.0));
            }
            if let Some((class, text)) = &self.last_failure {
                render_failure_box(ui, class, text);
            }

            let max_score = self
                .run_feed
                .iter()
                .filter(|e| e.status == "SUCCESS")
                .map(|e| e.score)
                .fold(0.0_f64, f64::max)
                .max(1e-9);
            egui::ScrollArea::vertical()
                .id_salt("live_run_feed")
                .max_height(170.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for (i, ev) in self.run_feed.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.monospace(format!("{:>3}", i + 1));
                            ui.label(
                                RichText::new(format!("{:<9}", ev.operator))
                                    .monospace()
                                    .color(operator_color(&ev.operator))
                                    .strong(),
                            );
                            let (rect, _) = ui
                                .allocate_exact_size(Vec2::new(200.0, 12.0), egui::Sense::hover());
                            let painter = ui.painter();
                            painter.rect_filled(
                                rect,
                                CornerRadius::same(3),
                                ui.visuals().extreme_bg_color,
                            );
                            let ok = ev.status == "SUCCESS";
                            if ok {
                                let frac = (ev.score / max_score).clamp(0.0, 1.0) as f32;
                                let fill = egui::Rect::from_min_max(
                                    rect.min,
                                    Pos2::new(rect.min.x + rect.width() * frac, rect.max.y),
                                );
                                painter.rect_filled(
                                    fill,
                                    CornerRadius::same(3),
                                    operator_color(&ev.operator),
                                );
                                ui.monospace(format!("{:.2} ({:+.2})", ev.score, ev.delta));
                            } else {
                                let mark = egui::Rect::from_min_max(
                                    rect.min,
                                    Pos2::new(rect.min.x + 6.0, rect.max.y),
                                );
                                painter.rect_filled(mark, CornerRadius::same(3), ACCENT_ROSE);
                                ui.monospace(ev.status.clone());
                            }
                            if ev.new_best {
                                ui.label(RichText::new("NEW BEST").color(ACCENT_EMERALD).strong());
                            }
                            ui.monospace(format!("{:.1}s", ev.secs));
                        });
                    }
                });
        });
        if stop_clicked {
            self.request_stop();
        }
        if clear_clicked {
            self.run_feed.clear();
        }
    }

    pub fn open_external(&mut self, rel_path: &str) {
        let p = self.repo_root.join(rel_path);
        if !p.exists() {
            self.status_banner = format!("Asset not found: {}", p.display());
            return;
        }
        let res = ProcessCommand::new("cmd")
            .args(["/C", "start", "", &p.to_string_lossy()])
            .spawn();
        match res {
            Ok(_) => self.status_banner = format!("Opened {}", p.display()),
            Err(e) => self.status_banner = format!("Failed to open {}: {e}", p.display()),
        }
    }
}

fn switch_on_hover(ctx: &egui::Context, titles: &[egui::Response]) {
    let is_open =
        |t: &egui::Response| egui::Popup::is_id_open(ctx, egui::Popup::default_response_id(t));
    if !titles.iter().any(is_open) {
        return;
    }
    if let Some(title) = titles
        .iter()
        .find(|t| t.hovered() && !t.clicked() && !is_open(t))
    {
        egui::Popup::open_id(ctx, egui::Popup::default_response_id(title));
    }
}

impl RsiCraftApp {
    /// Run one menu or command-palette id. Engine work goes through `Command::dispatch`.
    pub fn menu_action(&mut self, id: &str) -> Result<Value, String> {
        let go = |page: StudioPage| json!({ "page": page.label() });
        match id {
            "new_task" | "insert_spec" => {
                self.page = StudioPage::TaskAndSandboxLab;
                self.status_banner =
                    "Task lab open. Fill the specification and upsert it into the store.".into();
                Ok(go(self.page))
            }
            "open_db" => {
                if self.job.is_some() {
                    self.status_banner = "Open Database waits until the run finishes".into();
                } else {
                    self.pick_open = true;
                    self.status_banner = "Choose a SQLite database".into();
                }
                Ok(json!({ "db": "pick" }))
            }
            "save" => {
                if self.job.is_some() {
                    self.status_banner = "Save waits until the run finishes".into();
                    return Ok(json!({ "persisted": "busy" }));
                }
                if let Some(path) = self.ctx.reg.db_path().map(Path::to_path_buf) {
                    match self.ctx.reg.checkpoint() {
                        Ok(()) => {
                            self.status_banner = format!("Saved {}", path.display());
                            Ok(json!({ "persisted": path.display().to_string() }))
                        }
                        Err(e) => Err(e.to_string()),
                    }
                } else {
                    self.pick_save = true;
                    self.status_banner = "This store is in memory. Choose a file.".into();
                    Ok(json!({ "persisted": "pick" }))
                }
            }
            "save_as" => {
                if self.job.is_some() {
                    self.status_banner = "Save As waits until the run finishes".into();
                } else {
                    self.pick_save = true;
                    self.status_banner = "Choose where to save the database".into();
                }
                Ok(json!({ "persisted": "pick" }))
            }
            "export" => {
                let tid = self.active_task_id.clone();
                let v = self.exec(Command::Export { task_id: tid })?;
                let n = v
                    .get("cards")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0);
                self.status_banner = format!(
                    "Lineage bundle is in memory ({n} cards). `rsicraft export` writes the file."
                );
                Ok(v)
            }
            "close_task" => {
                self.selected_card_json = None;
                self.page = StudioPage::EvolutionaryStudio;
                self.status_banner = format!("Closed the card view for {}", self.active_task_id);
                Ok(go(self.page))
            }
            "quit" => {
                self.quit_requested = true;
                self.status_banner = "Quit requested".into();
                Ok(json!({ "quit": true }))
            }
            "undo" => self.undo_store(),
            "redo" => self.redo_store(),
            "cut" | "paste" | "select_all" => {
                self.status_banner =
                    "Cut, paste, and select all have no selection. Copy uses the open card.".into();
                Ok(json!({ "edit": id }))
            }
            "copy_card" => {
                let code = self
                    .selected_card_json
                    .as_ref()
                    .and_then(|v| v.get("code"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                match code {
                    Some(code) => {
                        self.pending_copy = Some(code);
                        self.status_banner = "Card source copied".into();
                        Ok(json!({ "copied": true }))
                    }
                    None => {
                        self.status_banner = "Open a card before copying its source".into();
                        Err("no card open".into())
                    }
                }
            }
            "preferences" | "view_gpu" => {
                self.page = StudioPage::Goliath4090AndLlm;
                self.status_banner = "LLM endpoint and model alias".into();
                Ok(go(self.page))
            }
            "view_lineage" => {
                self.page = StudioPage::LineageDagCanvas;
                Ok(go(self.page))
            }
            "view_cards" => {
                self.page = StudioPage::ExperienceCards;
                Ok(go(self.page))
            }
            "view_lab" => {
                self.page = StudioPage::TaskAndSandboxLab;
                Ok(go(self.page))
            }
            "view_studio" => {
                self.page = StudioPage::EvolutionaryStudio;
                Ok(go(self.page))
            }
            "view_help" => {
                self.page = StudioPage::MultitabHelpAndTools;
                Ok(go(self.page))
            }
            "view_dossier" => {
                self.page = StudioPage::RsiAndTsinghuaDossier;
                Ok(go(self.page))
            }
            "theme_dark" => {
                self.dark_mode = true;
                self.theme_dirty = true;
                self.status_banner = "Dark theme".into();
                Ok(json!({ "theme": "dark" }))
            }
            "theme_light" => {
                self.dark_mode = false;
                self.theme_dirty = true;
                self.status_banner = "Light theme".into();
                Ok(json!({ "theme": "light" }))
            }
            "zoom_reset" => {
                self.zoom_reset = true;
                self.status_banner = "Zoom reset to 100%".into();
                Ok(json!({ "zoom": 1.0 }))
            }
            "seed" => match self.undoable("seed", Command::DemoSeed) {
                Ok(v) => {
                    self.refresh_all();
                    self.status_banner = "Seeded the demo task".into();
                    Ok(v)
                }
                Err(e) => {
                    self.status_banner = e.clone();
                    Err(e)
                }
            },
            "step_improve" => {
                self.page = StudioPage::EvolutionaryStudio;
                self.trigger_step(SearchOperator::Improve);
                Ok(json!({ "op": "IMPROVE" }))
            }
            "step_crossover" => {
                self.page = StudioPage::EvolutionaryStudio;
                self.trigger_step(SearchOperator::Crossover);
                Ok(json!({ "op": "CROSSOVER" }))
            }
            "step_debug" => {
                self.page = StudioPage::EvolutionaryStudio;
                self.trigger_step(SearchOperator::Debug);
                Ok(json!({ "op": "DEBUG" }))
            }
            "prune" => {
                let tid = self.active_task_id.clone();
                let v = self.undoable(
                    "prune",
                    Command::CardsPrune {
                        task_id: tid,
                        keep_frontier: 10,
                    },
                )?;
                self.refresh_all();
                self.status_banner = "Pruned cards outside the frontier".into();
                Ok(v)
            }
            "hyperparams" => {
                let v = self.exec(Command::HyperparamsGet)?;
                self.status_banner = format!("Utility weights: {v}");
                Ok(v)
            }
            "campaign" => {
                self.page = StudioPage::EvolutionaryStudio;
                self.start_campaign();
                Ok(json!({ "banner": self.status_banner }))
            }
            "probe" => {
                let url = Some(self.llama_url_edit.clone());
                match self.exec(Command::LlmProbe { url }) {
                    Ok(v) => {
                        self.status_banner = "LLM probe reached the endpoint".into();
                        Ok(v)
                    }
                    Err(e) => {
                        self.status_banner = e.clone();
                        Err(e)
                    }
                }
            }
            "frontier" => {
                let tid = self.active_task_id.clone();
                let v = self.exec(Command::GetFrontier {
                    task_id: tid,
                    limit: 10,
                })?;
                self.frontier_json = v.clone();
                self.page = StudioPage::EvolutionaryStudio;
                self.status_banner = "Frontier refreshed".into();
                Ok(v)
            }
            "lineage" => {
                let tid = self.active_task_id.clone();
                let v = self.exec(Command::Lineage { task_id: tid })?;
                self.lineage_json = v.clone();
                Ok(v)
            }
            "tasks" => {
                let v = self.exec(Command::TaskList)?;
                self.tasks_json = v.clone();
                Ok(v)
            }
            "status" => {
                let v = self.exec(Command::Status)?;
                self.status_json = v.clone();
                Ok(v)
            }
            "hardware" => {
                let v = self.exec(Command::HardwareProfile)?;
                self.hw_json = v.clone();
                Ok(v)
            }
            "help_api" => {
                self.page = StudioPage::MultitabHelpAndTools;
                self.exec(Command::Help {
                    tab: Some("api".into()),
                    search: None,
                })
            }
            "reset_layout" => {
                self.page = StudioPage::EvolutionaryStudio;
                self.view_mode = ViewMode::Cards;
                self.show_quick_help_modal = false;
                self.status_banner = "Layout reset to the evolutionary studio".into();
                Ok(json!({ "layout": "default" }))
            }
            "palette" => {
                self.active_dialog = Some(StudioDialog::CommandPalette);
                Ok(json!({ "dialog": "palette" }))
            }
            "shortcuts" => {
                self.active_dialog = Some(StudioDialog::Shortcuts);
                Ok(json!({ "dialog": "shortcuts" }))
            }
            "docs" => {
                self.active_dialog = Some(StudioDialog::Docs);
                Ok(json!({ "dialog": "docs" }))
            }
            "licenses" => {
                self.active_dialog = Some(StudioDialog::Licenses);
                Ok(json!({ "dialog": "licenses" }))
            }
            "about" => {
                self.active_dialog = Some(StudioDialog::About);
                Ok(json!({ "dialog": "about" }))
            }
            other => Err(format!("unknown menu action '{other}'")),
        }
    }

    fn menu_item(&mut self, ui: &mut egui::Ui, id: &str, label: &str) {
        if ui.button(label).clicked() {
            match self.menu_action(id) {
                Ok(_) => {}
                Err(e) => self.status_banner = e,
            }
            ui.close();
        }
    }

    fn render_menu_bar(&mut self, ui: &mut egui::Ui) {
        let recent = self.open_recent.clone();
        egui::Panel::top("top_menubar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let file = ui
                    .menu_button("File", |ui| {
                        self.menu_item(ui, "new_task", "New Task... (Ctrl+N)");
                        self.menu_item(ui, "open_db", "Open Database... (Ctrl+O)");
                        if recent.is_empty() {
                            ui.label("Open Recent: none");
                        } else {
                            for (i, path) in recent.iter().enumerate() {
                                let label = format!("Recent: {}", path.display());
                                if ui.button(label).clicked() {
                                    self.pending_recent = Some(i);
                                    ui.close();
                                }
                            }
                        }
                        ui.separator();
                        self.menu_item(ui, "save", "Save (Ctrl+S)");
                        self.menu_item(ui, "save_as", "Save As... (Ctrl+Shift+S)");
                        ui.separator();
                        self.menu_item(ui, "export", "Export Lineage... (Ctrl+E)");
                        ui.separator();
                        self.menu_item(ui, "close_task", "Close Task (Ctrl+W)");
                        self.menu_item(ui, "quit", "Quit (Ctrl+Q)");
                    })
                    .response;
                let edit = ui
                    .menu_button("Edit", |ui| {
                        self.menu_item(ui, "undo", "Undo (Ctrl+Z)");
                        self.menu_item(ui, "redo", "Redo (Ctrl+Y)");
                        ui.separator();
                        self.menu_item(ui, "cut", "Cut");
                        self.menu_item(ui, "copy_card", "Copy card source (Ctrl+C)");
                        self.menu_item(ui, "paste", "Paste");
                        self.menu_item(ui, "select_all", "Select All");
                        ui.separator();
                        self.menu_item(ui, "preferences", "Preferences... (Ctrl+,)");
                    })
                    .response;
                let view = ui
                    .menu_button("View", |ui| {
                        self.menu_item(ui, "zoom_reset", "Reset Zoom (100%)");
                        ui.separator();
                        self.menu_item(ui, "view_studio", "Evolutionary Studio");
                        self.menu_item(ui, "view_lineage", "Lineage DAG Canvas");
                        self.menu_item(ui, "view_cards", "Experience Cards");
                        self.menu_item(ui, "view_lab", "Task and Sandbox Lab");
                        self.menu_item(ui, "view_gpu", "RTX 4090 and LLM");
                        self.menu_item(ui, "view_dossier", "What is RSI?");
                        self.menu_item(ui, "view_help", "Help and Tools");
                        ui.separator();
                        self.menu_item(ui, "theme_dark", "Theme: Dark");
                        self.menu_item(ui, "theme_light", "Theme: Light");
                    })
                    .response;
                let insert = ui
                    .menu_button("Insert", |ui| {
                        self.menu_item(ui, "seed", "Add Seed Candidate");
                        self.menu_item(ui, "insert_spec", "Insert Problem Specification");
                    })
                    .response;
                let modify = ui
                    .menu_button("Modify", |ui| {
                        self.menu_item(ui, "step_improve", "Mutate Candidate (IMPROVE)");
                        self.menu_item(ui, "step_debug", "Repair Candidate (DEBUG)");
                        self.menu_item(ui, "step_crossover", "Cross Parents (CROSSOVER)");
                        ui.separator();
                        self.menu_item(ui, "prune", "Prune Frontier");
                        self.menu_item(ui, "hyperparams", "Show Utility Weights");
                    })
                    .response;
                let commands = ui
                    .menu_button("Commands", |ui| {
                        self.menu_item(ui, "campaign", "Run Evolutionary Loop");
                        self.menu_item(ui, "probe", "Probe LLM Endpoint");
                        self.menu_item(ui, "view_lab", "Evaluate Candidate Sandbox");
                        self.menu_item(ui, "frontier", "Calculate Pareto Frontier");
                    })
                    .response;
                let window = ui
                    .menu_button("Window", |ui| {
                        self.menu_item(ui, "reset_layout", "Reset Panel Layout");
                        ui.separator();
                        self.menu_item(ui, "view_lineage", "Lineage DAG Viewport");
                        self.menu_item(ui, "view_cards", "Candidate Diff Inspector");
                        self.menu_item(ui, "view_studio", "Live Run");
                    })
                    .response;
                let help = ui
                    .menu_button("Help", |ui| {
                        self.menu_item(ui, "palette", "Command Palette... (Ctrl+K)");
                        self.menu_item(ui, "shortcuts", "Keyboard Shortcuts... (F1)");
                        self.menu_item(ui, "docs", "Project Files and Docs...");
                        self.menu_item(ui, "licenses", "Licenses and Notices...");
                        ui.separator();
                        self.menu_item(ui, "about", "About RSICraft");
                    })
                    .response;
                switch_on_hover(
                    ui.ctx(),
                    &[file, edit, view, insert, modify, commands, window, help],
                );
            });
        });
    }

    fn render_active_dialog(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.active_dialog else {
            return;
        };
        let title = match dialog {
            StudioDialog::CommandPalette => "Command Palette",
            StudioDialog::Shortcuts => "Keyboard Shortcuts",
            StudioDialog::Docs => "Project Files and Docs",
            StudioDialog::Licenses => "Licenses and Notices",
            StudioDialog::About => "About RSICraft",
        };
        let mut open = true;
        egui::Window::new(title).open(&mut open).show(ctx, |ui| {
            match dialog {
                StudioDialog::CommandPalette => {
                    ui.label("Filter, then run. Engine entries call Command::dispatch.");
                    ui.text_edit_singleline(&mut self.palette_query);
                    let q = self.palette_query.to_ascii_lowercase();
                    for (id, label, shortcut) in MENU_CATALOG {
                        if !q.is_empty()
                            && !label.to_ascii_lowercase().contains(&q)
                            && !id.contains(q.as_str())
                        {
                            continue;
                        }
                        let text = if shortcut.is_empty() {
                            (*label).to_string()
                        } else {
                            format!("{label}  ({shortcut})")
                        };
                        if ui.button(text).clicked() {
                            match self.menu_action(id) {
                                Ok(_) => {}
                                Err(e) => self.status_banner = e,
                            }
                            let opens_dialog = matches!(
                                *id,
                                "palette" | "shortcuts" | "docs" | "licenses" | "about"
                            );
                            if !opens_dialog {
                                self.active_dialog = None;
                            }
                        }
                    }
                }
                StudioDialog::Shortcuts => {
                    for (id, label, shortcut) in MENU_CATALOG {
                        if shortcut.is_empty() {
                            continue;
                        }
                        ui.label(format!("{shortcut}    {label}    [{id}]"));
                    }
                }
                StudioDialog::Docs => {
                    ui.label("Opens the file with the system handler.");
                    for rel in [
                        "docs/PRD.md",
                        "docs/ARCHITECTURE.md",
                        "docs/TIERED_PLAN.md",
                        "HANDOVER.md",
                        "README.md",
                    ] {
                        if ui.button(rel).clicked() {
                            self.open_external(rel);
                        }
                    }
                }
                StudioDialog::Licenses => {
                    ui.label("RSICraft is MIT OR Apache-2.0.");
                    ui.label(
                        "Workspace clippy denies unwrap, expect, panic, todo, and unimplemented.",
                    );
                    ui.label(
                        "The crates do not declare forbid(unsafe_code). Dependencies include egui, eframe, rfd, rusqlite, serde, blake3, zstd, clap, and ureq.",
                    );
                }
                StudioDialog::About => {
                    ui.heading(format!("RSICraft {VERSION}"));
                    ui.label("Core engine, 26-tool CLI and MCP server, egui studio.");
                    ui.label(
                        "Search is measured. No generation has been trained. The menu bar is this window's tier-4 shell.",
                    );
                }
            }
        });
        if !open {
            self.active_dialog = None;
        }
    }
}

fn harsh_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::from_rgb(10, 13, 18); // slate-950
    v.window_fill = Color32::from_rgb(15, 23, 42);
    v.extreme_bg_color = Color32::from_rgb(6, 9, 15);
    v.override_text_color = Some(Color32::from_rgb(245, 247, 250));
    v.hyperlink_color = ACCENT_CYAN;
    v.widgets.noninteractive.bg_fill = Color32::from_rgb(16, 26, 46);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(38, 54, 84));
    v.widgets.inactive.bg_fill = Color32::from_rgb(22, 33, 56);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(48, 66, 102));
    v.widgets.hovered.bg_fill = Color32::from_rgb(30, 46, 76);
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ACCENT_CYAN);
    v.widgets.active.bg_fill = Color32::from_rgb(14, 116, 144);
    v.selection.bg_fill = Color32::from_rgb(3, 105, 161);
    v
}

fn crisp_light_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    v.panel_fill = Color32::WHITE;
    v.window_fill = Color32::WHITE;
    v.extreme_bg_color = Color32::WHITE;
    v.faint_bg_color = Color32::WHITE;
    v.override_text_color = Some(Color32::BLACK);
    v.weak_text_color = Some(Color32::BLACK);
    v.widgets.noninteractive.bg_fill = Color32::WHITE;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::BLACK);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::BLACK);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(203, 213, 225));
    v.widgets.inactive.bg_fill = Color32::from_rgb(226, 232, 240);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(148, 163, 184));
    v.widgets.hovered.bg_fill = Color32::from_rgb(224, 242, 254);
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, Color32::from_rgb(2, 132, 199));
    v
}

fn configure_typography(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.text_styles.insert(
            egui::TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Body,
            FontId::new(15.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            FontId::new(14.5, FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            FontId::new(13.5, FontFamily::Monospace),
        );
        style.spacing.item_spacing = Vec2::new(8.0, 7.0);
    });
}

fn handle_zoom(ctx: &egui::Context) {
    let scroll_y = ctx.input(|i| {
        if i.modifiers.ctrl || i.modifiers.command {
            i.smooth_scroll_delta.y
        } else {
            0.0
        }
    });
    if scroll_y.abs() > 0.0 {
        let step = if scroll_y > 0.0 {
            1.1_f32
        } else {
            1.0 / 1.1_f32
        };
        let next = (ctx.zoom_factor() * step).clamp(0.6, 2.5);
        ctx.set_zoom_factor(next);
    }
}

fn theme_toggle_button(ui: &mut egui::Ui, dark_mode: &mut bool) -> bool {
    let size = Vec2::new(28.0, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&resp);
        ui.painter().rect(
            rect,
            CornerRadius::same(6),
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let center = rect.center();
        let painter = ui.painter();
        if *dark_mode {
            let sun_color = ACCENT_AMBER;
            painter.circle_filled(center, 4.2, sun_color);
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let dir = Vec2::new(angle.cos(), angle.sin());
                painter.line_segment(
                    [center + dir * 6.0, center + dir * 8.6],
                    Stroke::new(1.5, sun_color),
                );
            }
        } else {
            let moon_color = Color32::from_rgb(51, 65, 85);
            painter.circle_filled(center, 6.2, moon_color);
            painter.circle_filled(center + Vec2::new(2.8, -2.0), 5.2, visuals.bg_fill);
        }
    }
    let clicked = resp
        .on_hover_text("Toggle Dark (slate-950) / Light (slate-50) theme")
        .clicked();
    if clicked {
        *dark_mode = !*dark_mode;
        ui.ctx().set_visuals(if *dark_mode {
            harsh_visuals()
        } else {
            crisp_light_visuals()
        });
    }
    clicked
}

fn paint_status_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 14.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.2, color);
}

fn accent_card(ui: &mut egui::Ui, stripe_color: Color32, add_contents: impl FnOnce(&mut egui::Ui)) {
    let frame = egui::Frame::group(ui.style())
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin {
            left: 14,
            right: 12,
            top: 10,
            bottom: 10,
        });
    let resp = frame.show(ui, add_contents);
    let r = resp.response.rect;
    let stripe = egui::Rect::from_min_max(r.min, Pos2::new(r.min.x + 4.0, r.max.y));
    ui.painter()
        .rect_filled(stripe, CornerRadius::same(2), stripe_color);
}

fn operator_color(op: &str) -> Color32 {
    match op {
        "DRAFT" => ACCENT_CYAN,
        "IMPROVE" => ACCENT_EMERALD,
        "DEBUG" => ACCENT_AMBER,
        "CROSSOVER" => ACCENT_PURPLE,
        _ => Color32::from_rgb(148, 163, 184),
    }
}

#[derive(Clone, Copy)]
struct Point3D {
    x: f32,
    y: f32,
    z: f32,
}

impl Point3D {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn project(
        &self,
        rot_x: f32,
        rot_y: f32,
        zoom: f32,
        center: Pos2,
        fov: f32,
        cam_dist: f32,
    ) -> Option<(Pos2, f32)> {
        let cos_y = rot_y.cos();
        let sin_y = rot_y.sin();
        let cos_x = rot_x.cos();
        let sin_x = rot_x.sin();

        // Yaw around Y
        let x1 = self.x * cos_y + self.z * sin_y;
        let y1 = self.y;
        let z1 = -self.x * sin_y + self.z * cos_y;

        // Pitch around X
        let x2 = x1;
        let y2 = y1 * cos_x - z1 * sin_x;
        let z2 = y1 * sin_x + z1 * cos_x;

        let depth = cam_dist + z2;
        if depth > 10.0 {
            let scale = (fov * zoom) / depth;
            let sx = center.x + x2 * scale;
            let sy = center.y - y2 * scale;
            Some((Pos2::new(sx, sy), z2))
        } else {
            None
        }
    }
}

fn draw_sparkle_star(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    color: Color32,
    angle: f32,
) {
    if radius <= 0.8 {
        return;
    }
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let arm_long = radius;
    let arm_short = radius * 0.35;

    let p_top = Pos2::new(center.x - sin_a * arm_long, center.y - cos_a * arm_long);
    let p_bottom = Pos2::new(center.x + sin_a * arm_long, center.y + cos_a * arm_long);
    let p_right = Pos2::new(center.x + cos_a * arm_long, center.y - sin_a * arm_long);
    let p_left = Pos2::new(center.x - cos_a * arm_long, center.y + sin_a * arm_long);

    let stroke = Stroke::new(1.3, color);
    painter.line_segment([p_left, p_right], stroke);
    painter.line_segment([p_top, p_bottom], stroke);

    let cos_d = (angle + std::f32::consts::FRAC_PI_4).cos();
    let sin_d = (angle + std::f32::consts::FRAC_PI_4).sin();
    let d_top = Pos2::new(center.x - sin_d * arm_short, center.y - cos_d * arm_short);
    let d_bottom = Pos2::new(center.x + sin_d * arm_short, center.y + cos_d * arm_short);
    let d_right = Pos2::new(center.x + cos_d * arm_short, center.y - sin_d * arm_short);
    let d_left = Pos2::new(center.x - cos_d * arm_short, center.y + sin_d * arm_short);

    let dim_color = Color32::from_rgba_premultiplied(
        color.r(),
        color.g(),
        color.b(),
        (color.a() as f32 * 0.5) as u8,
    );
    let dim_stroke = Stroke::new(1.0, dim_color);
    painter.line_segment([d_left, d_right], dim_stroke);
    painter.line_segment([d_top, d_bottom], dim_stroke);

    painter.circle_filled(center, (radius * 0.25).max(1.0), Color32::WHITE);
}

impl RsiCraftApp {
    pub fn render_rtx_4090_3d_viewport(&mut self, ui: &mut egui::Ui, viewport_height: f32) {
        let (rect, resp) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(380.0), viewport_height),
            egui::Sense::click_and_drag(),
        );

        if resp.dragged() {
            let delta = resp.drag_delta();
            self.gpu_rot_y += delta.x * 0.012;
            self.gpu_rot_x = (self.gpu_rot_x - delta.y * 0.012).clamp(-1.45, 1.45);
        } else if self.gpu_auto_spin {
            self.gpu_rot_y += self.gpu_spin_speed;
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(6, 9, 15));
        painter.rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(1.2, Color32::from_rgb(30, 48, 75)),
            egui::StrokeKind::Inside,
        );

        // Cybernetic background grid dots
        let grid_gap = 36.0_f32;
        let x_start = rect.left() + (rect.width() % grid_gap) * 0.5;
        let mut x = x_start;
        while x < rect.right() {
            let mut y = rect.top() + 12.0;
            while y < rect.bottom() - 12.0 {
                painter.circle_filled(
                    Pos2::new(x, y),
                    1.0,
                    Color32::from_rgba_premultiplied(56, 189, 248, 28),
                );
                y += grid_gap;
            }
            x += grid_gap;
        }

        let center = rect.center();
        let fov = 420.0;
        let cam_dist = 520.0;

        let proj = |pt: Point3D| -> Option<(Pos2, f32)> {
            pt.project(
                self.gpu_rot_x,
                self.gpu_rot_y,
                self.gpu_zoom,
                center,
                fov,
                cam_dist,
            )
        };

        // Style colors
        let (frame_col, fin_col, _accent_light, fan_blade_col) = match self.gpu_style {
            1 => (
                Color32::from_rgb(148, 163, 184), // Founders Matte Gunmetal
                Color32::from_rgb(71, 85, 105),
                Color32::from_rgb(226, 232, 240),
                Color32::from_rgb(100, 116, 139),
            ),
            2 => (
                ACCENT_EMERALD, // Wireframe Holo
                Color32::from_rgb(16, 185, 129),
                ACCENT_EMERALD,
                Color32::from_rgb(5, 150, 105),
            ),
            _ => (
                ACCENT_CYAN, // Cyberpunk Neon
                Color32::from_rgb(30, 58, 95),
                ACCENT_PURPLE,
                Color32::from_rgb(56, 189, 248),
            ),
        };

        // 1. Shroud Bounding Box & Chamfers (RTX 4090 3.5-Slot Form Factor)
        let hx = 125.0;
        let hy = 50.0;
        let hz = 25.0;

        let corners = [
            Point3D::new(-hx, -hy, -hz), // 0: LBB
            Point3D::new(hx, -hy, -hz),  // 1: RBB
            Point3D::new(hx, hy, -hz),   // 2: RTB
            Point3D::new(-hx, hy, -hz),  // 3: LTB
            Point3D::new(-hx, -hy, hz),  // 4: LBF
            Point3D::new(hx, -hy, hz),   // 5: RBF
            Point3D::new(hx, hy, hz),    // 6: RTF
            Point3D::new(-hx, hy, hz),   // 7: LTF
        ];

        let proj_corners: Vec<Option<(Pos2, f32)>> = corners.iter().map(|c| proj(*c)).collect();

        let draw_edge = |i: usize, j: usize, stroke: Stroke| {
            if let (Some(Some((p1, _))), Some(Some((p2, _)))) =
                (proj_corners.get(i), proj_corners.get(j))
            {
                painter.line_segment([*p1, *p2], stroke);
            }
        };

        let frame_stroke = Stroke::new(2.0, frame_col);
        // Back rect
        draw_edge(0, 1, frame_stroke);
        draw_edge(1, 2, frame_stroke);
        draw_edge(2, 3, frame_stroke);
        draw_edge(3, 0, frame_stroke);
        // Front rect
        draw_edge(4, 5, frame_stroke);
        draw_edge(5, 6, frame_stroke);
        draw_edge(6, 7, frame_stroke);
        draw_edge(7, 4, frame_stroke);
        // Connecting struts
        draw_edge(0, 4, frame_stroke);
        draw_edge(1, 5, frame_stroke);
        draw_edge(2, 6, frame_stroke);
        draw_edge(3, 7, frame_stroke);

        // 2. Aluminum Heatsink Fin Array (Transverse Cooling Fins)
        let fin_stroke = Stroke::new(1.0, fin_col);
        for k in 0..15 {
            let fx = -105.0 + (k as f32) * 15.0;
            if (fx - (-65.0)).abs() > 28.0 && (fx - 65.0).abs() > 28.0 {
                if let (Some((top_pt, _)), Some((bot_pt, _))) = (
                    proj(Point3D::new(fx, hy - 4.0, hz)),
                    proj(Point3D::new(fx, -hy + 4.0, hz)),
                ) {
                    painter.line_segment([top_pt, bot_pt], fin_stroke);
                }
            }
        }

        // 3. Dual Contra-Rotating Axial Flow Fans (Flow-Through Architecture)
        let fan_centers = [(-62.0_f32, 1.0_f32), (62.0_f32, -1.0_f32)];
        for (fan_idx, (cx, dir)) in fan_centers.iter().enumerate() {
            let fan_rad = 38.0;
            // Fan ring (16 segments)
            let mut ring_pts: Vec<Pos2> = Vec::new();
            for seg in 0..16 {
                let theta = (seg as f32) * std::f32::consts::TAU / 16.0;
                let px = cx + theta.cos() * fan_rad;
                let py = theta.sin() * fan_rad;
                if let Some((p, _)) = proj(Point3D::new(px, py, hz + 2.0)) {
                    ring_pts.push(p);
                }
            }
            if ring_pts.len() >= 3 {
                ring_pts.push(ring_pts[0]);
                let halo_col = if fan_idx == 0 {
                    ACCENT_CYAN
                } else {
                    ACCENT_PURPLE
                };
                painter.add(egui::Shape::line(ring_pts, Stroke::new(1.6, halo_col)));
            }

            // Central fan hub
            if let Some((hub_pt, _)) = proj(Point3D::new(*cx, 0.0, hz + 2.0)) {
                painter.circle_filled(hub_pt, 7.0 * self.gpu_zoom, Color32::from_rgb(15, 23, 42));
                painter.circle_stroke(
                    hub_pt,
                    7.0 * self.gpu_zoom,
                    Stroke::new(1.2, ACCENT_EMERALD),
                );
            }

            // 9 Contra-Rotating Fan Blades
            let base_angle = (self.anim_tick as f32 * 0.12 * dir) % std::f32::consts::TAU;
            for b in 0..9 {
                let angle = base_angle + (b as f32) * std::f32::consts::TAU / 9.0;
                let bx = cx + angle.cos() * (fan_rad - 2.0);
                let by = angle.sin() * (fan_rad - 2.0);
                if let (Some((hub_pt, _)), Some((tip_pt, _))) = (
                    proj(Point3D::new(*cx, 0.0, hz + 2.0)),
                    proj(Point3D::new(bx, by, hz + 1.5)),
                ) {
                    painter.line_segment([hub_pt, tip_pt], Stroke::new(1.8, fan_blade_col));
                }
            }
        }

        // 4. Central AD102 Die & 12x GDDR6X Micron VRAM Chips (PCB Backplate View)
        if self.gpu_show_vram_chips {
            // Central Silicon Die (AD102-300-A1)
            let die_hw = 18.0_f32;
            let die_corners = [
                Point3D::new(-die_hw, -die_hw, -hz - 1.0),
                Point3D::new(die_hw, -die_hw, -hz - 1.0),
                Point3D::new(die_hw, die_hw, -hz - 1.0),
                Point3D::new(-die_hw, die_hw, -hz - 1.0),
            ];
            let die_proj: Vec<Pos2> = die_corners
                .iter()
                .filter_map(|p| proj(*p).map(|(pos, _)| pos))
                .collect();
            if die_proj.len() == 4 {
                painter.add(egui::Shape::convex_polygon(
                    die_proj,
                    Color32::from_rgb(18, 25, 38),
                    Stroke::new(1.6, Color32::from_rgb(234, 179, 8)),
                ));
                if let Some((die_center, _)) = proj(Point3D::new(0.0, 0.0, -hz - 1.0)) {
                    painter.text(
                        die_center,
                        egui::Align2::CENTER_CENTER,
                        "AD102",
                        FontId::monospace(9.0),
                        Color32::WHITE,
                    );
                }
            }

            // 12 VRAM Chips (2 GB each): 10 Active (19.1 GB used), 2 Free (4.9 GB headroom)
            let chip_coords = [
                (-32.0, 30.0),
                (0.0, 30.0),
                (32.0, 30.0), // Top 3
                (-32.0, -30.0),
                (0.0, -30.0),
                (32.0, -30.0), // Bottom 3
                (-40.0, 14.0),
                (-40.0, -14.0), // Left 2
                (40.0, 14.0),
                (40.0, -14.0), // Right 2
                (-40.0, 0.0),
                (40.0, 0.0), // Extra 2
            ];

            for (i, (vx, vy)) in chip_coords.iter().enumerate() {
                let is_active = i < 10;
                let chip_col = if is_active {
                    Color32::from_rgb(16, 185, 129) // Emerald 2GB Active
                } else {
                    Color32::from_rgb(51, 65, 85) // Slate 2GB Free Safety Margin
                };
                let cw = 7.0_f32;
                let ch = 7.0_f32;
                let chip_pts = [
                    Point3D::new(vx - cw, vy - ch, -hz - 1.0),
                    Point3D::new(vx + cw, vy - ch, -hz - 1.0),
                    Point3D::new(vx + cw, vy + ch, -hz - 1.0),
                    Point3D::new(vx - cw, vy + ch, -hz - 1.0),
                ];
                let p_chip: Vec<Pos2> = chip_pts
                    .iter()
                    .filter_map(|p| proj(*p).map(|(pos, _)| pos))
                    .collect();
                if p_chip.len() == 4 {
                    painter.add(egui::Shape::convex_polygon(
                        p_chip,
                        chip_col,
                        Stroke::new(1.0, Color32::from_rgb(148, 163, 184)),
                    ));
                }
            }
        }

        // 5. PCIe 4.0 x16 Interface (Gold Fingers along bottom edge)
        if let (Some((p_start, _)), Some((p_end, _))) = (
            proj(Point3D::new(-110.0, -hy, 0.0)),
            proj(Point3D::new(-15.0, -hy, 0.0)),
        ) {
            painter.line_segment(
                [p_start, p_end],
                Stroke::new(3.2, Color32::from_rgb(234, 179, 8)),
            );
        }

        // 6. 12VHPWR 16-Pin Power Header (Top edge)
        if let Some((pwr_pt, _)) = proj(Point3D::new(60.0, hy, 0.0)) {
            painter.circle_filled(pwr_pt, 4.0 * self.gpu_zoom, Color32::from_rgb(34, 197, 94));
        }

        // 7. "GEFORCE RTX" Illuminated Side Plate
        if let Some((logo_pt, _)) = proj(Point3D::new(0.0, hy + 2.0, 0.0)) {
            let glow = ((self.anim_tick as f32 * 0.08).sin() * 0.3 + 0.7).clamp(0.4, 1.0);
            let logo_col = Color32::from_rgba_premultiplied(
                (56.0 * glow) as u8,
                (189.0 * glow) as u8,
                (248.0 * glow) as u8,
                255,
            );
            painter.text(
                logo_pt,
                egui::Align2::CENTER_CENTER,
                "GEFORCE RTX 4090",
                FontId::proportional(11.0 * self.gpu_zoom),
                logo_col,
            );
        }

        // 8. Plasma Sparks & Particle Glints
        if self.gpu_plasma_fx {
            let spark_count = 10;
            for s in 0..spark_count {
                let t_spark =
                    (self.anim_tick as f32 * 0.04 + (s as f32) * 0.628) % std::f32::consts::TAU;
                let sx = t_spark.cos() * 95.0;
                let sy = t_spark.sin() * 45.0 + ((s as f32 * 7.0).sin() * 15.0);
                let sz = ((s as f32 * 2.3).cos()) * 35.0;
                if let Some((spark_pos, _)) = proj(Point3D::new(sx, sy, sz)) {
                    let col = if s % 2 == 0 {
                        ACCENT_CYAN
                    } else {
                        ACCENT_AMBER
                    };
                    draw_sparkle_star(&painter, spark_pos, 6.5, col, t_spark * 1.5);
                }
            }
        }

        // 9. Floating On-Screen HUD Overlay & Controls
        let hud_rect = egui::Rect::from_min_size(
            Pos2::new(rect.left() + 10.0, rect.bottom() - 36.0),
            Vec2::new(rect.width() - 20.0, 28.0),
        );
        painter.rect_filled(
            hud_rect,
            CornerRadius::same(6),
            Color32::from_rgba_premultiplied(10, 15, 26, 215),
        );
        painter.rect_stroke(
            hud_rect,
            CornerRadius::same(6),
            Stroke::new(1.0, Color32::from_rgb(30, 48, 75)),
            egui::StrokeKind::Inside,
        );

        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(hud_rect));
        child_ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.gpu_auto_spin,
                    if self.gpu_auto_spin {
                        "[Auto-Spin: ON]"
                    } else {
                        "[Auto-Spin: OFF]"
                    },
                )
                .clicked()
            {
                self.gpu_auto_spin = !self.gpu_auto_spin;
            }
            if ui
                .small_button(match self.gpu_style {
                    1 => "Style: Founders",
                    2 => "Style: Wireframe",
                    _ => "Style: Cyberpunk",
                })
                .clicked()
            {
                self.gpu_style = (self.gpu_style + 1) % 3;
            }
            if ui
                .selectable_label(self.gpu_show_vram_chips, "12x VRAM")
                .clicked()
            {
                self.gpu_show_vram_chips = !self.gpu_show_vram_chips;
            }
            if ui
                .selectable_label(self.gpu_plasma_fx, "Plasma FX")
                .clicked()
            {
                self.gpu_plasma_fx = !self.gpu_plasma_fx;
            }
            ui.separator();
            if ui.small_button("Reset").clicked() {
                self.gpu_rot_x = -0.32;
                self.gpu_rot_y = 0.65;
                self.gpu_zoom = 1.0;
            }
            if ui.small_button("Fans").clicked() {
                self.gpu_rot_x = 0.0;
                self.gpu_rot_y = 0.0;
            }
            if ui.small_button("PCB/Core").clicked() {
                self.gpu_rot_x = 0.0;
                self.gpu_rot_y = std::f32::consts::PI;
            }
            ui.separator();
            let yaw_deg = (self.gpu_rot_y.to_degrees() % 360.0).round() as i32;
            let pitch_deg = self.gpu_rot_x.to_degrees().round() as i32;
            ui.label(
                RichText::new(format!(
                    "Yaw: {yaw_deg} deg | Pitch: {pitch_deg} deg | 19.1/24.0 GB"
                ))
                .size(11.0)
                .monospace()
                .color(ACCENT_EMERALD),
            );
        });
    }

    pub fn render_rsi_3d_landscape_viewport(&mut self, ui: &mut egui::Ui, viewport_height: f32) {
        let (rect, resp) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(420.0), viewport_height),
            egui::Sense::click_and_drag(),
        );

        if resp.dragged() {
            let delta = resp.drag_delta();
            self.rsi_3d_rot_y += delta.x * 0.012;
            self.rsi_3d_rot_x = (self.rsi_3d_rot_x - delta.y * 0.012).clamp(-1.45, 1.45);
        } else if self.rsi_3d_auto_spin {
            self.rsi_3d_rot_y += 0.007;
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(6, 9, 15));
        painter.rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(1.2, Color32::from_rgb(30, 48, 75)),
            egui::StrokeKind::Inside,
        );

        let center = rect.center();
        let fov = 440.0;
        let cam_dist = 560.0;

        let proj = |pt: Point3D| -> Option<(Pos2, f32)> {
            pt.project(
                self.rsi_3d_rot_x,
                self.rsi_3d_rot_y,
                self.rsi_3d_zoom,
                center,
                fov,
                cam_dist,
            )
        };

        // 1. Cybernetic Ground Grid (Generation vs Novelty Floor)
        let floor_y = -65.0_f32;
        let grid_lines = 10;
        let extent_x = 130.0_f32;
        let extent_z = 90.0_f32;

        let grid_stroke = Stroke::new(1.0, Color32::from_rgba_premultiplied(30, 58, 95, 110));
        for i in 0..=grid_lines {
            let frac = (i as f32) / (grid_lines as f32);
            let gx = -extent_x + frac * (2.0 * extent_x);
            if let (Some((p1, _)), Some((p2, _))) = (
                proj(Point3D::new(gx, floor_y, -extent_z)),
                proj(Point3D::new(gx, floor_y, extent_z)),
            ) {
                painter.line_segment([p1, p2], grid_stroke);
            }

            let gz = -extent_z + frac * (2.0 * extent_z);
            if let (Some((p1, _)), Some((p2, _))) = (
                proj(Point3D::new(-extent_x, floor_y, gz)),
                proj(Point3D::new(extent_x, floor_y, gz)),
            ) {
                painter.line_segment([p1, p2], grid_stroke);
            }
        }

        // 2. Coordinate Axis Labels in 3D
        if let Some((p_x, _)) = proj(Point3D::new(extent_x + 14.0, floor_y, 0.0)) {
            painter.text(
                p_x,
                egui::Align2::LEFT_CENTER,
                "Generations (t) ->",
                FontId::monospace(10.0),
                ACCENT_CYAN,
            );
        }
        if let Some((p_y, _)) = proj(Point3D::new(-extent_x - 10.0, 70.0, 0.0)) {
            painter.text(
                p_y,
                egui::Align2::RIGHT_CENTER,
                "^ Fitness Score (S)",
                FontId::monospace(10.0),
                ACCENT_EMERALD,
            );
        }
        if let Some((p_z, _)) = proj(Point3D::new(0.0, floor_y, extent_z + 14.0)) {
            painter.text(
                p_z,
                egui::Align2::CENTER_TOP,
                "Novelty / Delta (Delta) ->",
                FontId::monospace(10.0),
                ACCENT_AMBER,
            );
        }

        // 3. Extract experience cards
        let cards = self
            .cards_json
            .get("cards")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut chronological = cards;
        chronological.reverse();

        if !chronological.is_empty() {
            let min_s = chronological
                .iter()
                .filter_map(|c| c.get("primary_score").and_then(Value::as_f64))
                .fold(f64::INFINITY, f64::min);
            let max_s = chronological
                .iter()
                .filter_map(|c| c.get("primary_score").and_then(Value::as_f64))
                .fold(f64::NEG_INFINITY, f64::max);
            let span_s = (max_s - min_s).abs().max(1.0);
            let denom = (chronological.len().saturating_sub(1)).max(1) as f32;

            // Map each card to 3D point (X=step, Y=score, Z=utility/novelty)
            let mut mapped_nodes: Vec<(Point3D, Pos2, String, String, f64, f64)> = Vec::new();

            for (idx, c) in chronological.iter().enumerate() {
                let cid = c
                    .get("card_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let op = c
                    .get("operator")
                    .and_then(Value::as_str)
                    .unwrap_or("DRAFT")
                    .to_string();
                let sc = c
                    .get("primary_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let ut = c
                    .get("composite_utility")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);

                let tx = (idx as f32) / denom;
                let ty = ((sc - min_s) / span_s) as f32;
                let tz = (ut as f32).clamp(0.0, 1.0);

                let x_3d = -110.0 + tx * 220.0;
                let y_3d = floor_y + 12.0 + ty * 115.0;
                let z_3d = -60.0 + tz * 120.0;

                let pt = Point3D::new(x_3d, y_3d, z_3d);
                if let Some((pos_2d, _)) = proj(pt) {
                    mapped_nodes.push((pt, pos_2d, cid, op, sc, ut));
                }
            }

            // 4. 3D Trajectory Splines Connecting Nodes
            if mapped_nodes.len() >= 2 {
                for i in 0..(mapped_nodes.len() - 1) {
                    let (_, p0, _, _, _, _) = mapped_nodes[i];
                    let (_, p1, _, op_next, _, _) = &mapped_nodes[i + 1];
                    let col = operator_color(op_next);
                    painter.line_segment([p0, *p1], Stroke::new(2.4, col));

                    // Animated streaming 3D particle along segment
                    let t_pulse =
                        ((self.anim_tick as f32 * 0.03 + (i as f32) * 0.25) % 1.0).clamp(0.0, 1.0);
                    let pulse_pos = Pos2::new(
                        p0.x * (1.0 - t_pulse) + p1.x * t_pulse,
                        p0.y * (1.0 - t_pulse) + p1.y * t_pulse,
                    );
                    painter.circle_filled(pulse_pos, 4.0, col);
                    painter.circle_filled(pulse_pos, 2.0, Color32::WHITE);
                }
            }

            // 5. Draw 3D Card Nodes
            for (idx, (pt_3d, pos_2d, _cid, op, sc, _ut)) in mapped_nodes.iter().enumerate() {
                let is_peak = (sc - max_s).abs() < 1e-4 && max_s > 0.0;
                let col = if is_peak {
                    ACCENT_AMBER
                } else {
                    operator_color(op)
                };

                // Drop line down to grid floor
                if let Some((floor_pos, _)) = proj(Point3D::new(pt_3d.x, floor_y, pt_3d.z)) {
                    painter.line_segment(
                        [*pos_2d, floor_pos],
                        Stroke::new(1.0, Color32::from_rgba_premultiplied(71, 85, 105, 90)),
                    );
                    painter.circle_filled(
                        floor_pos,
                        2.0,
                        Color32::from_rgba_premultiplied(56, 189, 248, 60),
                    );
                }

                painter.circle_filled(*pos_2d, if is_peak { 7.5 } else { 5.5 }, col);
                painter.circle_stroke(
                    *pos_2d,
                    if is_peak { 7.5 } else { 5.5 },
                    Stroke::new(1.4, Color32::WHITE),
                );

                if is_peak {
                    let pulse = ((self.anim_tick as f32 * 0.08).sin() * 2.5).abs();
                    draw_sparkle_star(
                        &painter,
                        *pos_2d,
                        12.0 + pulse,
                        ACCENT_AMBER,
                        self.anim_tick as f32 * 0.05,
                    );

                    // Floating 3D Hypercube wireframe around peak summit
                    let cube_rad = 12.0_f32;
                    let c_pts = [
                        Point3D::new(pt_3d.x - cube_rad, pt_3d.y - cube_rad, pt_3d.z - cube_rad),
                        Point3D::new(pt_3d.x + cube_rad, pt_3d.y - cube_rad, pt_3d.z - cube_rad),
                        Point3D::new(pt_3d.x + cube_rad, pt_3d.y + cube_rad, pt_3d.z - cube_rad),
                        Point3D::new(pt_3d.x - cube_rad, pt_3d.y + cube_rad, pt_3d.z - cube_rad),
                        Point3D::new(pt_3d.x - cube_rad, pt_3d.y - cube_rad, pt_3d.z + cube_rad),
                        Point3D::new(pt_3d.x + cube_rad, pt_3d.y - cube_rad, pt_3d.z + cube_rad),
                        Point3D::new(pt_3d.x + cube_rad, pt_3d.y + cube_rad, pt_3d.z + cube_rad),
                        Point3D::new(pt_3d.x - cube_rad, pt_3d.y + cube_rad, pt_3d.z + cube_rad),
                    ];
                    let cp: Vec<Option<(Pos2, f32)>> = c_pts.iter().map(|p| proj(*p)).collect();
                    let d_c = |i: usize, j: usize| {
                        if let (Some(Some((a, _))), Some(Some((b, _)))) = (cp.get(i), cp.get(j)) {
                            painter.line_segment([*a, *b], Stroke::new(1.2, ACCENT_AMBER));
                        }
                    };
                    d_c(0, 1);
                    d_c(1, 2);
                    d_c(2, 3);
                    d_c(3, 0);
                    d_c(4, 5);
                    d_c(5, 6);
                    d_c(6, 7);
                    d_c(7, 4);
                    d_c(0, 4);
                    d_c(1, 5);
                    d_c(2, 6);
                    d_c(3, 7);

                    let badge_rect = egui::Rect::from_min_size(
                        Pos2::new(pos_2d.x - 52.0, pos_2d.y - 25.0),
                        Vec2::new(104.0, 16.0),
                    );
                    painter.rect_filled(
                        badge_rect,
                        CornerRadius::same(3),
                        Color32::from_rgb(15, 23, 42),
                    );
                    painter.rect_stroke(
                        badge_rect,
                        CornerRadius::same(3),
                        Stroke::new(1.0, ACCENT_AMBER),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("PEAK: {sc:.2} (Gen {idx})"),
                        FontId::monospace(9.0),
                        ACCENT_AMBER,
                    );
                } else {
                    painter.text(
                        *pos_2d + Vec2::new(8.0, -8.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("{op} ({sc:.2})"),
                        FontId::proportional(10.0),
                        col,
                    );
                }
            }
        }

        // 6. Floating On-Screen Controls HUD
        let hud_rect = egui::Rect::from_min_size(
            Pos2::new(rect.left() + 10.0, rect.bottom() - 36.0),
            Vec2::new(rect.width() - 20.0, 28.0),
        );
        painter.rect_filled(
            hud_rect,
            CornerRadius::same(6),
            Color32::from_rgba_premultiplied(10, 15, 26, 220),
        );
        painter.rect_stroke(
            hud_rect,
            CornerRadius::same(6),
            Stroke::new(1.0, Color32::from_rgb(30, 48, 75)),
            egui::StrokeKind::Inside,
        );

        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(hud_rect));
        child_ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.rsi_3d_auto_spin,
                    if self.rsi_3d_auto_spin {
                        "[Auto-Spin: ON]"
                    } else {
                        "[Auto-Spin: OFF]"
                    },
                )
                .clicked()
            {
                self.rsi_3d_auto_spin = !self.rsi_3d_auto_spin;
            }
            if ui.small_button("Reset Orbit").clicked() {
                self.rsi_3d_rot_x = -0.38;
                self.rsi_3d_rot_y = 0.52;
                self.rsi_3d_zoom = 1.0;
            }
            if ui.small_button("Top (XY)").clicked() {
                self.rsi_3d_rot_x = -1.45;
                self.rsi_3d_rot_y = 0.0;
            }
            if ui.small_button("Side (YZ)").clicked() {
                self.rsi_3d_rot_x = 0.0;
                self.rsi_3d_rot_y = std::f32::consts::FRAC_PI_2;
            }
            ui.separator();
            let yaw_deg = (self.rsi_3d_rot_y.to_degrees() % 360.0).round() as i32;
            let pitch_deg = self.rsi_3d_rot_x.to_degrees().round() as i32;
            ui.label(
                RichText::new(format!(
                    "Yaw: {yaw_deg} deg | Pitch: {pitch_deg} deg | 3D Evolutionary Fitness Manifold"
                ))
                .size(11.0)
                .monospace()
                .color(ACCENT_CYAN),
            );
        });
    }

    fn render_kpi_ribbon(&self, ui: &mut egui::Ui) {
        let tasks_cnt = self
            .status_json
            .get("tasks_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let cards_cnt = self
            .status_json
            .get("cards_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let succ_cnt = self
            .status_json
            .get("success_cards_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let hashes_cnt = self
            .status_json
            .get("unique_code_hashes")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let best_score = self
            .frontier_json
            .get("frontier")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|c| c.get("primary_score"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);

        ui.columns(4, |cols| {
            accent_card(&mut cols[0], ACCENT_CYAN, |ui| {
                ui.label(
                    RichText::new("PARETO FRONTIER LEADER")
                        .size(11.5)
                        .color(ACCENT_CYAN)
                        .strong(),
                );
                ui.label(
                    RichText::new(format!("{best_score:.4} ({})", self.active_task_id))
                        .size(18.0)
                        .strong(),
                );
                ui.label(RichText::new("Verified 4090 Peak: 96.6028 (1.359 ms)").size(12.0));
            });
            accent_card(&mut cols[1], ACCENT_EMERALD, |ui| {
                ui.label(
                    RichText::new("STIGMERGIC WAL + BLAKE3/ZSTD CAS")
                        .size(11.5)
                        .color(ACCENT_EMERALD)
                        .strong(),
                );
                ui.label(
                    RichText::new(format!(
                        "{cards_cnt} Cards | {succ_cnt} Success | {hashes_cnt} Hashes"
                    ))
                    .size(16.0)
                    .strong(),
                );
                ui.label(
                    RichText::new(format!("{tasks_cnt} Tasks in SQLite WAL Store")).size(12.0),
                );
            });
            accent_card(&mut cols[2], ACCENT_AMBER, |ui| {
                ui.label(
                    RichText::new("GOLIATH RTX 4090 (24 GB VRAM)")
                        .size(11.5)
                        .color(ACCENT_AMBER)
                        .strong(),
                );
                ui.label(
                    RichText::new("Frontis-MA1 30B/35B MoE (~3B Active)")
                        .size(15.5)
                        .strong(),
                );
                ui.label(
                    RichText::new("18.0-19.5 GB Bounded | 16k q4_0 KV | ~120 tok/s").size(12.0),
                );
            });
            accent_card(&mut cols[3], ACCENT_PURPLE, |ui| {
                ui.label(
                    RichText::new("TSINGHUA / OPENRSI UTILITY MATH")
                        .size(11.5)
                        .color(ACCENT_PURPLE)
                        .strong(),
                );
                ui.label(
                    RichText::new("U_i = 0.50*s + 0.35*d + 0.15*nu")
                        .monospace()
                        .size(15.0)
                        .strong(),
                );
                ui.label(RichText::new("arXiv:2607.28568 | FrontisAI/OpenRSI").size(12.0));
            });
        });
    }

    fn render_score_spark_chart(&self, ui: &mut egui::Ui) {
        accent_card(ui, ACCENT_CYAN, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(
                        "Evolutionary Trajectory Spark Chart (Primary Score vs Card Index)",
                    )
                    .strong(),
                );
                ui.label(
                    RichText::new("(Hover data points to inspect operator & composite utility)")
                        .size(12.5),
                );
            });

            let cards = self
                .cards_json
                .get("cards")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            let desired = Vec2::new(ui.available_width().max(320.0), 145.0);
            let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::hover());
            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(8, 12, 20));
            let grid_stroke = Stroke::new(1.0, Color32::from_rgb(30, 41, 59));
            for frac in [0.2_f32, 0.5, 0.8] {
                let y = egui::lerp(rect.top() + 10.0..=rect.bottom() - 16.0, frac);
                painter.line_segment(
                    [
                        Pos2::new(rect.left() + 8.0, y),
                        Pos2::new(rect.right() - 8.0, y),
                    ],
                    grid_stroke,
                );
            }

            if cards.is_empty() {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No cards recorded yet. Click [Seed Demo] or [DRAFT] above.",
                    FontId::proportional(14.0),
                    Color32::WHITE,
                );
                return;
            }

            let mut chronological = cards;
            chronological.reverse();

            let min_s = chronological
                .iter()
                .filter_map(|c| c.get("primary_score").and_then(Value::as_f64))
                .fold(f64::INFINITY, f64::min);
            let max_s = chronological
                .iter()
                .filter_map(|c| c.get("primary_score").and_then(Value::as_f64))
                .fold(f64::NEG_INFINITY, f64::max);
            let span = (max_s - min_s).abs().max(1.0);
            let plot = rect.shrink2(Vec2::new(18.0, 18.0));
            let denom = (chronological.len().saturating_sub(1)).max(1) as f32;

            let pts: Vec<(Pos2, &Value)> = chronological
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let s = c
                        .get("primary_score")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let tx = (i as f32) / denom;
                    let ty = ((s - min_s) / span) as f32;
                    let pos = Pos2::new(
                        egui::lerp(plot.left()..=plot.right(), tx),
                        egui::lerp(plot.bottom()..=plot.top(), ty),
                    );
                    (pos, c)
                })
                .collect();

            let line_pts: Vec<Pos2> = pts.iter().map(|(p, _)| *p).collect();
            if line_pts.len() >= 2 {
                // Translucent gradient trapezoid area fill below curve
                for w in line_pts.windows(2) {
                    let p0 = w[0];
                    let p1 = w[1];
                    let quad = vec![
                        p0,
                        p1,
                        Pos2::new(p1.x, plot.bottom()),
                        Pos2::new(p0.x, plot.bottom()),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        quad,
                        Color32::from_rgba_premultiplied(14, 165, 233, 24),
                        Stroke::NONE,
                    ));
                }

                // Glowing main trajectory line
                painter.add(egui::Shape::line(
                    line_pts.clone(),
                    Stroke::new(2.4, ACCENT_CYAN),
                ));

                // Laser comet pulse moving along trajectory
                let t_comet = ((self.anim_tick as f32 * 0.015) % 1.0).clamp(0.0, 1.0);
                let seg_count = line_pts.len() - 1;
                let seg_f = t_comet * (seg_count as f32);
                let idx = (seg_f.floor() as usize).min(seg_count - 1);
                let local_t = seg_f - idx as f32;
                if let (Some(p0), Some(p1)) = (line_pts.get(idx), line_pts.get(idx + 1)) {
                    let comet_pos = Pos2::new(
                        p0.x * (1.0 - local_t) + p1.x * local_t,
                        p0.y * (1.0 - local_t) + p1.y * local_t,
                    );
                    painter.circle_filled(comet_pos, 5.0, Color32::from_rgb(224, 242, 254));
                    painter.circle_stroke(comet_pos, 8.0, Stroke::new(1.4, ACCENT_CYAN));
                }
            }

            // Data point dots
            for (pt, c) in &pts {
                let op = c.get("operator").and_then(Value::as_str).unwrap_or("DRAFT");
                let col = operator_color(op);
                painter.circle_filled(*pt, 5.0, col);
                painter.circle_stroke(*pt, 5.0, Stroke::new(1.2, Color32::WHITE));
            }

            // Peak frontier star sparkle
            if let Some((max_pt, max_c)) = pts.iter().max_by(|(_, a), (_, b)| {
                let sa = a
                    .get("primary_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let sb = b
                    .get("primary_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                sa.total_cmp(&sb)
            }) {
                let pulse = ((self.anim_tick as f32 * 0.08).sin() * 2.5).abs();
                painter.circle_stroke(*max_pt, 7.5 + pulse, Stroke::new(1.8, ACCENT_AMBER));
                draw_sparkle_star(
                    &painter,
                    *max_pt,
                    11.5 + pulse,
                    ACCENT_AMBER,
                    self.anim_tick as f32 * 0.05,
                );

                let max_s = max_c
                    .get("primary_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let badge_rect = egui::Rect::from_min_size(
                    Pos2::new(max_pt.x - 45.0, max_pt.y - 23.0),
                    Vec2::new(90.0, 15.0),
                );
                painter.rect_filled(
                    badge_rect,
                    CornerRadius::same(3),
                    Color32::from_rgb(15, 23, 42),
                );
                painter.rect_stroke(
                    badge_rect,
                    CornerRadius::same(3),
                    Stroke::new(1.0, ACCENT_AMBER),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("FRONTIER: {max_s:.2}"),
                    FontId::monospace(9.0),
                    ACCENT_AMBER,
                );
            }

            if let Some(mouse) = resp.hover_pos() {
                if let Some((pt, c)) = pts.iter().min_by(|(a, _), (b, _)| {
                    (a.x - mouse.x).abs().total_cmp(&(b.x - mouse.x).abs())
                }) {
                    painter.vline(pt.x, plot.y_range(), Stroke::new(1.0, ACCENT_CYAN));
                    let op = c.get("operator").and_then(Value::as_str).unwrap_or("?");
                    let sc = c
                        .get("primary_score")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let ut = c
                        .get("composite_utility")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let cid = c.get("card_id").and_then(Value::as_str).unwrap_or("?");
                    resp.on_hover_text(format!(
                        "Card: {}\nOperator: {op}\nScore: {sc:.4}\nComposite Utility: {ut:.4}",
                        &cid[..cid.len().min(8)]
                    ));
                }
            }
        });
    }

    fn render_evolutionary_studio(&mut self, ui: &mut egui::Ui) {
        accent_card(ui, ACCENT_EMERALD, |ui| {
            ui.heading(
                "4-Operator Evolutionary Action Space Launchpad (Frontis-MA1 / OpenMLE-Evo)",
            );
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Task ID:").strong());
                ui.add(egui::TextEdit::singleline(&mut self.active_task_id).desired_width(120.0));

                ui.label(RichText::new("Provider:").strong());
                egui::ComboBox::from_id_salt("prov_combo")
                    .selected_text(&self.provider_pick)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut self.provider_pick,
                            "demo".to_string(),
                            "demo (Deterministic Oracle)",
                        );
                        let _ = ui.selectable_value(
                            &mut self.provider_pick,
                            "llama".to_string(),
                            "llama (RTX 4090 llama-server)",
                        );
                    });

                ui.label(RichText::new("Sandbox:").strong());
                egui::ComboBox::from_id_salt("sb_combo")
                    .selected_text(&self.sandbox_pick)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_value(
                            &mut self.sandbox_pick,
                            "demo".to_string(),
                            "demo (In-Memory Oracle)",
                        );
                        let _ = ui.selectable_value(
                            &mut self.sandbox_pick,
                            "local".to_string(),
                            "local (Isolated Python 3.12)",
                        );
                        let _ = ui.selectable_value(
                            &mut self.sandbox_pick,
                            "docker".to_string(),
                            "docker (Air-Gapped Container)",
                        );
                    });

                if ui.button("Refresh Task").clicked() {
                    self.refresh_all();
                }
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(
                        RichText::new("[+ DRAFT (T=0.70)]")
                            .color(ACCENT_CYAN)
                            .strong(),
                    )
                    .on_hover_text("Generate novel baseline architecture from task specification")
                    .clicked()
                {
                    self.trigger_step(SearchOperator::Draft);
                }
                if ui
                    .button(
                        RichText::new("[+ IMPROVE (T=0.40)]")
                            .color(ACCENT_EMERALD)
                            .strong(),
                    )
                    .on_hover_text("Sample 1 functional parent via U_i roulette and optimize")
                    .clicked()
                {
                    self.trigger_step(SearchOperator::Improve);
                }
                if ui
                    .button(
                        RichText::new("[+ DEBUG (T=0.20)]")
                            .color(ACCENT_AMBER)
                            .strong(),
                    )
                    .on_hover_text("Sample 1 broken parent and inject stack trace to repair")
                    .clicked()
                {
                    self.trigger_step(SearchOperator::Debug);
                }
                if ui
                    .button(
                        RichText::new("[+ CROSSOVER (T=0.50)]")
                            .color(ACCENT_PURPLE)
                            .strong(),
                    )
                    .on_hover_text("Sample 2 distinct high-utility parents and recombine")
                    .clicked()
                {
                    self.trigger_step(SearchOperator::Crossover);
                }
                ui.separator();
                if ui
                    .button(RichText::new("[>> Run 4-Step Campaign]").strong())
                    .clicked()
                {
                    self.start_campaign();
                }
            });
        });

        self.render_live_run_panel(ui);
        ui.add_space(6.0);
        self.render_score_spark_chart(ui);
        ui.add_space(6.0);

        egui::CollapsingHeader::new(
            RichText::new(
                "Interactive 3D Hardware Telemetry: NVIDIA GeForce RTX 4090 (24 GB VRAM)",
            )
            .strong()
            .color(ACCENT_AMBER),
        )
        .default_open(true)
        .show(ui, |ui| {
            self.render_rtx_4090_3d_viewport(ui, 250.0);
        });

        ui.add_space(6.0);

        ui.heading(
            "Materialized Top-K Pareto Frontier (Ranked by Primary Score & Composite Utility)",
        );
        let frontier = self
            .frontier_json
            .get("frontier")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut inspect_id: Option<String> = None;
        for item in &frontier {
            let rank = item.get("rank").and_then(Value::as_u64).unwrap_or(0);
            let cid = item.get("card_id").and_then(Value::as_str).unwrap_or("?");
            let op = item.get("operator").and_then(Value::as_str).unwrap_or("?");
            let status = item.get("status").and_then(Value::as_str).unwrap_or("?");
            let score = item
                .get("primary_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let delta = item
                .get("delta_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let util = item
                .get("composite_utility")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let ms = item
                .get("execution_time_ms")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let hash = item.get("code_hash").and_then(Value::as_str).unwrap_or("?");

            accent_card(ui, operator_color(op), |ui| {
                ui.horizontal_wrapped(|ui| {
                    paint_status_dot(ui, operator_color(op));
                    ui.label(
                        RichText::new(format!("Rank #{rank} | {op}"))
                            .color(operator_color(op))
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!("Card {}", &cid[..cid.len().min(8)])).monospace(),
                    );
                    ui.separator();
                    ui.label(RichText::new(format!("Score: {score:.4}")).strong());
                    ui.label(format!("Delta: {delta:+.4}"));
                    ui.label(format!("Time: {ms} ms"));
                    ui.label(format!("Status: {status}"));
                    ui.label(
                        RichText::new(format!("BLAKE3: {}...", &hash[..hash.len().min(12)]))
                            .monospace(),
                    );
                    if ui.small_button("[Inspect Code & CAS]").clicked() {
                        inspect_id = Some(cid.to_string());
                    }
                });
                ui.add(
                    egui::ProgressBar::new((util as f32).clamp(0.0, 1.0))
                        .fill(operator_color(op))
                        .text(format!(
                            "Composite Utility U_i = {util:.4} (0.50*s_norm + 0.35*d_norm + 0.15*nu_i)"
                        )),
                );
            });
        }
        if let Some(id) = inspect_id {
            self.selected_card_json = self.exec(Command::CardShow { card_id: id }).ok();
            self.page = StudioPage::ExperienceCards;
        }
    }

    fn render_lineage_dag_canvas(&mut self, ui: &mut egui::Ui) {
        self.anim_tick = self.anim_tick.wrapping_add(1);
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));

        ui.horizontal_wrapped(|ui| {
            ui.heading("Evolutionary Lineage & Fitness Architecture");
            ui.separator();
            if ui
                .selectable_label(!self.dag_3d_mode, "[2D Bezier Node Graph]")
                .clicked()
            {
                self.dag_3d_mode = false;
            }
            if ui
                .selectable_label(
                    self.dag_3d_mode,
                    "[3D Fitness Manifold & Search Trajectory]",
                )
                .clicked()
            {
                self.dag_3d_mode = true;
            }
            ui.separator();
            paint_status_dot(ui, ACCENT_CYAN);
            ui.label("DRAFT");
            paint_status_dot(ui, ACCENT_EMERALD);
            ui.label("IMPROVE");
            paint_status_dot(ui, ACCENT_AMBER);
            ui.label("DEBUG");
            paint_status_dot(ui, ACCENT_PURPLE);
            ui.label("CROSSOVER");
        });

        if self.dag_3d_mode {
            self.render_rsi_3d_landscape_viewport(ui, 420.0);
            ui.add_space(6.0);
            self.render_selected_card_inspector(ui);
            return;
        }

        let nodes = self
            .lineage_json
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let edges = self
            .lineage_json
            .get("edges")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let desired = Vec2::new(ui.available_width().max(480.0), 390.0);
        let (response, painter) = ui.allocate_painter(desired, egui::Sense::click());
        let origin = response.rect.min;

        painter.rect_filled(
            response.rect,
            CornerRadius::same(8),
            Color32::from_rgb(8, 12, 20),
        );
        painter.rect_stroke(
            response.rect,
            CornerRadius::same(8),
            Stroke::new(1.0, Color32::from_rgb(38, 54, 84)),
            egui::StrokeKind::Inside,
        );

        // Background cybernetic grid dots
        let grid_gap = 40.0_f32;
        let mut gx = response.rect.left() + 20.0;
        while gx < response.rect.right() - 10.0 {
            let mut gy = response.rect.top() + 20.0;
            while gy < response.rect.bottom() - 10.0 {
                painter.circle_filled(
                    Pos2::new(gx, gy),
                    1.0,
                    Color32::from_rgba_premultiplied(56, 189, 248, 25),
                );
                gy += grid_gap;
            }
            gx += grid_gap;
        }

        let mut ordered = nodes;
        ordered.reverse();

        // Assign deterministic grid coordinates for each node
        let mut positions: std::collections::HashMap<String, egui::Rect> =
            std::collections::HashMap::new();
        let cols = 4usize;
        for (idx, n) in ordered.iter().enumerate() {
            let cid = n
                .get("card_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let col = idx % cols;
            let row = idx / cols;
            let x = 28.0 + (col as f32) * 245.0;
            let y = 26.0 + (row as f32) * 115.0;
            let rect = egui::Rect::from_min_size(origin + Vec2::new(x, y), Vec2::new(205.0, 82.0));
            positions.insert(cid, rect);
        }

        // Draw Bezier edges with multi-particle streaming pulses
        for e in &edges {
            let from = e.get("from").and_then(Value::as_str).unwrap_or("");
            let to = e.get("to").and_then(Value::as_str).unwrap_or("");
            let op = e
                .get("operator")
                .and_then(Value::as_str)
                .unwrap_or("IMPROVE");
            if let (Some(r_from), Some(r_to)) = (positions.get(from), positions.get(to)) {
                let p0 = Pos2::new(r_from.right(), r_from.center().y);
                let p3 = Pos2::new(r_to.left(), r_to.center().y);
                let dx = ((p3.x - p0.x).abs() * 0.45).max(40.0);
                let p1 = p0 + Vec2::new(dx, 0.0);
                let p2 = p3 - Vec2::new(dx, 0.0);
                let col = operator_color(op);
                let bez = egui::epaint::CubicBezierShape::from_points_stroke(
                    [p0, p1, p2, p3],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(2.2, col),
                );
                painter.add(bez);

                // 3 Staggered streaming pulse particles per edge
                for offset in [0.0_f32, 0.333, 0.667] {
                    let t = ((self.anim_tick as f32 * 0.02 + offset) % 1.0).clamp(0.0, 1.0);
                    let omt = 1.0 - t;
                    let px = p0.x * omt.powi(3)
                        + p1.x * 3.0 * omt.powi(2) * t
                        + p2.x * 3.0 * omt * t.powi(2)
                        + p3.x * t.powi(3);
                    let py = p0.y * omt.powi(3)
                        + p1.y * 3.0 * omt.powi(2) * t
                        + p2.y * 3.0 * omt * t.powi(2)
                        + p3.y * t.powi(3);
                    let p_pos = Pos2::new(px, py);
                    painter.circle_filled(p_pos, 4.2, col);
                    painter.circle_filled(p_pos, 2.2, Color32::WHITE);
                }
            }
        }

        // Draw nodes & detect click with sparkling frontier nodes
        let max_score = ordered
            .iter()
            .filter_map(|n| n.get("primary_score").and_then(Value::as_f64))
            .fold(f64::NEG_INFINITY, f64::max);
        let mut clicked_card: Option<String> = None;
        let click_pos = if response.clicked() {
            response.interact_pointer_pos()
        } else {
            None
        };

        for n in &ordered {
            let cid = n.get("card_id").and_then(Value::as_str).unwrap_or("");
            let op = n.get("operator").and_then(Value::as_str).unwrap_or("DRAFT");
            let st = n.get("status").and_then(Value::as_str).unwrap_or("?");
            let sc = n
                .get("primary_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let ut = n
                .get("composite_utility")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if let Some(rect) = positions.get(cid) {
                let is_frontier = (sc - max_score).abs() < 1e-4 && max_score > 0.0;
                let col = if is_frontier {
                    ACCENT_AMBER
                } else {
                    operator_color(op)
                };

                painter.rect_filled(*rect, CornerRadius::same(8), Color32::from_rgb(17, 25, 42));
                painter.rect_stroke(
                    *rect,
                    CornerRadius::same(8),
                    Stroke::new(if is_frontier { 2.4 } else { 1.6 }, col),
                    egui::StrokeKind::Inside,
                );

                if is_frontier {
                    // Shimmering second halo & corner sparkle star
                    painter.rect_stroke(
                        rect.expand(2.0),
                        CornerRadius::same(10),
                        Stroke::new(1.0, Color32::from_rgba_premultiplied(251, 191, 36, 90)),
                        egui::StrokeKind::Outside,
                    );
                    draw_sparkle_star(
                        &painter,
                        Pos2::new(rect.right() - 8.0, rect.top() + 8.0),
                        8.0,
                        ACCENT_AMBER,
                        self.anim_tick as f32 * 0.06,
                    );
                }

                painter.text(
                    rect.min + Vec2::new(10.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    format!("{op} | {}", &cid[..cid.len().min(8)]),
                    FontId::proportional(13.5),
                    col,
                );
                painter.text(
                    rect.min + Vec2::new(10.0, 32.0),
                    egui::Align2::LEFT_TOP,
                    format!("Score: {sc:.4} ({st})"),
                    FontId::proportional(12.5),
                    Color32::WHITE,
                );
                painter.text(
                    rect.min + Vec2::new(10.0, 54.0),
                    egui::Align2::LEFT_TOP,
                    format!("Utility U_i: {ut:.4}"),
                    FontId::monospace(11.5),
                    Color32::from_rgb(186, 230, 253),
                );

                if let Some(cp) = click_pos {
                    if rect.contains(cp) {
                        clicked_card = Some(cid.to_string());
                    }
                }
            }
        }

        if let Some(cid) = clicked_card {
            self.selected_card_json = self.exec(Command::CardShow { card_id: cid }).ok();
        }

        ui.add_space(6.0);
        self.render_selected_card_inspector(ui);
    }

    fn render_selected_card_inspector(&mut self, ui: &mut egui::Ui) {
        let Some(card) = self.selected_card_json.clone() else {
            ui.label("Click any card or DAG node to inspect decompressed BLAKE3+Zstd source code.");
            return;
        };
        let cid = card
            .get("card_id")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();
        let op = card
            .get("operator_used")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();
        let hash = card
            .get("code_hash")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();
        let code = card
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let out = card
            .get("raw_output")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let sc = card
            .get("primary_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let ut = card
            .get("composite_utility")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let parent_a = card
            .get("parent_a_id")
            .and_then(Value::as_str)
            .map(str::to_string);

        let mut trigger_diff = None;

        accent_card(ui, operator_color(&op), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("Decompressed Artifact Inspector: Card {cid}"))
                        .color(operator_color(&op))
                        .strong(),
                );
                ui.separator();
                ui.label(format!("Operator: {op} | Score: {sc:.4} | U_i: {ut:.4}"));
                if let Some(pa) = &parent_a {
                    ui.separator();
                    let pa_short = &pa[..pa.len().min(8)];
                    if ui
                        .small_button(format!("[Diff with Parent A: {pa_short}...]"))
                        .clicked()
                    {
                        trigger_diff = Some((pa.clone(), cid.clone()));
                    }
                }
            });
            ui.label(
                RichText::new(format!("BLAKE3-256 Digest (Zstd L3 CAS): {hash}"))
                    .monospace()
                    .size(12.0),
            );
            if self.card_diff_output.is_some() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Line-by-Line Unified Diff (Parent -> Candidate):")
                            .color(ACCENT_AMBER)
                            .strong(),
                    );
                    if ui.small_button("[Close Diff]").clicked() {
                        self.card_diff_output = None;
                    }
                });
                if let Some(diff) = &self.card_diff_output {
                    ui.code(diff);
                }
            }
            ui.columns(2, |cols| {
                cols[0].label(RichText::new("Synthesized Source Code:").strong());
                cols[0].code(&code);
                cols[1].label(RichText::new("Sandbox Evaluator Output / Stack Trace:").strong());
                cols[1].code(&out);
            });
        });

        if let Some((pa, cb)) = trigger_diff {
            let cmd = Command::Diff {
                card_a_id: pa,
                card_b_id: cb,
            };
            if let Ok(v) = self.exec(cmd) {
                self.card_diff_output = v
                    .get("diff_text")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
        }
    }

    fn render_experience_cards_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Experience Cards & BLAKE3+Zstd Content-Addressable Store");
            ui.separator();
            if ui
                .selectable_label(self.view_mode == ViewMode::Cards, "[Cards (Default)]")
                .clicked()
            {
                self.view_mode = ViewMode::Cards;
            }
            if ui
                .selectable_label(self.view_mode == ViewMode::Table, "[Table]")
                .clicked()
            {
                self.view_mode = ViewMode::Table;
            }
            if ui
                .selectable_label(self.view_mode == ViewMode::Json, "[{ } JSON]")
                .clicked()
            {
                self.view_mode = ViewMode::Json;
            }
            ui.separator();
            ui.label("Search:");
            ui.add(egui::TextEdit::singleline(&mut self.card_search).desired_width(140.0));
        });

        let cards = self
            .cards_json
            .get("cards")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let needle = self.card_search.trim().to_ascii_lowercase();
        let filtered: Vec<&Value> = cards
            .iter()
            .filter(|c| {
                if needle.is_empty() {
                    true
                } else {
                    c.to_string().to_ascii_lowercase().contains(&needle)
                }
            })
            .collect();

        match self.view_mode {
            ViewMode::Json => {
                ui.code(serde_json::to_string_pretty(&self.cards_json).unwrap_or_default());
            }
            ViewMode::Table => {
                egui::Grid::new("cards_table_grid")
                    .striped(true)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Card ID").strong());
                        ui.label(RichText::new("Operator").strong());
                        ui.label(RichText::new("Status").strong());
                        ui.label(RichText::new("Score").strong());
                        ui.label(RichText::new("Delta").strong());
                        ui.label(RichText::new("Utility U_i").strong());
                        ui.label(RichText::new("Action").strong());
                        ui.end_row();

                        let mut pick = None;
                        for c in &filtered {
                            let cid = c.get("card_id").and_then(Value::as_str).unwrap_or("?");
                            let op = c.get("operator").and_then(Value::as_str).unwrap_or("?");
                            let st = c.get("status").and_then(Value::as_str).unwrap_or("?");
                            let sc = c
                                .get("primary_score")
                                .and_then(Value::as_f64)
                                .unwrap_or(0.0);
                            let dl = c.get("delta_score").and_then(Value::as_f64).unwrap_or(0.0);
                            let ut = c
                                .get("composite_utility")
                                .and_then(Value::as_f64)
                                .unwrap_or(0.0);
                            ui.monospace(&cid[..cid.len().min(8)]);
                            ui.colored_label(operator_color(op), op);
                            ui.label(st);
                            ui.monospace(format!("{sc:.4}"));
                            ui.monospace(format!("{dl:+.4}"));
                            ui.monospace(format!("{ut:.4}"));
                            if ui.small_button("[Inspect]").clicked() {
                                pick = Some(cid.to_string());
                            }
                            ui.end_row();
                        }
                        if let Some(id) = pick {
                            self.selected_card_json =
                                self.exec(Command::CardShow { card_id: id }).ok();
                        }
                    });
            }
            ViewMode::Cards => {
                let mut pick = None;
                for c in &filtered {
                    let cid = c.get("card_id").and_then(Value::as_str).unwrap_or("?");
                    let op = c.get("operator").and_then(Value::as_str).unwrap_or("?");
                    let st = c.get("status").and_then(Value::as_str).unwrap_or("?");
                    let sc = c
                        .get("primary_score")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let dl = c.get("delta_score").and_then(Value::as_f64).unwrap_or(0.0);
                    let ut = c
                        .get("composite_utility")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let hash = c.get("code_hash").and_then(Value::as_str).unwrap_or("?");
                    accent_card(ui, operator_color(op), |ui| {
                        ui.horizontal_wrapped(|ui| {
                            paint_status_dot(ui, operator_color(op));
                            ui.label(RichText::new(op).color(operator_color(op)).strong());
                            ui.monospace(&cid[..cid.len().min(8)]);
                            ui.label(format!(
                                "Status: {st} | Score: {sc:.4} | Delta: {dl:+.4} | U_i: {ut:.4}"
                            ));
                            ui.monospace(format!("BLAKE3: {}...", &hash[..hash.len().min(12)]));
                            if ui.small_button("[Decompress & View]").clicked() {
                                pick = Some(cid.to_string());
                            }
                        });
                    });
                }
                if let Some(id) = pick {
                    self.selected_card_json = self.exec(Command::CardShow { card_id: id }).ok();
                }
            }
        }

        ui.add_space(6.0);
        self.render_selected_card_inspector(ui);
    }

    fn render_task_and_sandbox_lab(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            accent_card(&mut cols[0], ACCENT_CYAN, |ui| {
                ui.heading("Register / Update Optimization Task");
                ui.horizontal(|ui| {
                    ui.label("Task ID:");
                    ui.text_edit_singleline(&mut self.edit_task_id);
                });
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.edit_task_name);
                });
                ui.label("Problem Specification:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.edit_task_desc)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                ui.label("Evaluator Script (must print RSI_METRIC_JSON={...}):");
                ui.add(
                    egui::TextEdit::multiline(&mut self.edit_evaluator)
                        .code_editor()
                        .desired_rows(5)
                        .desired_width(f32::INFINITY),
                );
                if ui.button("[Save Task to SQLite WAL Store]").clicked() {
                    let cmd = Command::TaskUpsert {
                        task_id: self.edit_task_id.clone(),
                        name: self.edit_task_name.clone(),
                        description: self.edit_task_desc.clone(),
                        evaluator_script: self.edit_evaluator.clone(),
                    };
                    match self.undoable("task", cmd) {
                        Ok(_) => {
                            self.active_task_id = self.edit_task_id.clone();
                            self.status_banner = format!("Upserted task '{}'", self.edit_task_id);
                            self.refresh_all();
                        }
                        Err(e) => self.status_banner = format!("Task upsert error: {e}"),
                    }
                }
            });

            accent_card(&mut cols[1], ACCENT_EMERALD, |ui| {
                ui.heading("Live Sandbox Candidate Evaluator");
                ui.label(format!(
                    "Active Task: '{}' | Sandbox Mode: '{}'",
                    self.active_task_id, self.sandbox_pick
                ));
                ui.add(
                    egui::TextEdit::multiline(&mut self.lab_candidate_code)
                        .code_editor()
                        .desired_rows(8)
                        .desired_width(f32::INFINITY),
                );
                if ui.button("[Evaluate & Record Candidate Card]").clicked() {
                    self.start_evaluate();
                }
                if let Some(ev) = &self.lab_last_eval {
                    ui.separator();
                    ui.code(serde_json::to_string_pretty(ev).unwrap_or_default());
                }
            });
        });
    }

    fn render_goliath_4090_and_llm(&mut self, ui: &mut egui::Ui) {
        accent_card(ui, ACCENT_AMBER, |ui| {
            ui.heading("Goliath Hardware Profile: NVIDIA GeForce RTX 4090 (24.0 GB GDDR6X)");
            ui.label(
                "Host: AMD Ryzen 9 7900X (24 Logical Threads) | 64 GB DDR5 | Bounded 18.0-19.5 GB VRAM Allocation",
            );
            ui.add_space(4.0);
            ui.add(egui::ProgressBar::new(19.1 / 24.0).fill(ACCENT_AMBER).text(
                "19.1 GB / 24.0 GB Allocated (4.9 GB Free Safety Margin - Zero OOM Thrashing)",
            ));
            ui.columns(3, |cols| {
                cols[0].label(RichText::new("Model Weights (16.8 GB)").strong());
                cols[0].label("Frontis-MA1 30B/35B MoE (~3B active/token) Q4_K_M / Q5_K_M");
                cols[1].label(RichText::new("16k KV Cache (1.5 GB)").strong());
                cols[1].label("--cache-type-k q4_0 --cache-type-v q4_0 --flash-attn");
                cols[2].label(RichText::new("CUDA Scratch (0.8 GB)").strong());
                cols[2].label("--parallel 2 continuous batching (~120 tok/s)");
            });
            if let Some(cmd) = self
                .hw_json
                .get("recommended_llama_server_cmd")
                .and_then(Value::as_str)
            {
                ui.add_space(4.0);
                ui.label(RichText::new("Recommended llama-server Launch Command:").strong());
                ui.code(cmd);
            }
        });

        ui.add_space(8.0);
        accent_card(ui, ACCENT_CYAN, |ui| {
            ui.heading("Interactive 3D Hardware Studio: NVIDIA GeForce RTX 4090 (AD102-300)");
            ui.label(
                "Real-time 3D GPU model showing dual contra-rotating flow-through fans, heatsink fin stack, 12x GDDR6X VRAM chips, PCIe 4.0 interface, and plasma emission (Click-and-drag to orbit in 3D).",
            );
            ui.add_space(4.0);
            self.render_rtx_4090_3d_viewport(ui, 340.0);
        });

        ui.add_space(8.0);
        accent_card(ui, ACCENT_AMBER, |ui| {
            ui.heading("Model server (provider \"llama\")");
            ui.label(
                "Any OpenAI-compatible server: Ollama, LM Studio, llama-server, vLLM or a hosted API. \
                 Enter the server address (not the full /v1/chat/completions path) and the model name \
                 exactly as the server lists it, apply, then check. Help page, tab \"Connecting a model\", \
                 has the details.",
            );
            ui.horizontal_wrapped(|ui| {
                ui.label("Server address:");
                ui.add(egui::TextEdit::singleline(&mut self.llama_url_edit).desired_width(260.0));
                ui.label("Model:");
                ui.add(egui::TextEdit::singleline(&mut self.model_alias_edit).desired_width(200.0));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Presets (address only; keep or type the model name):");
                for (label, url) in [
                    ("Ollama :11434", "http://127.0.0.1:11434"),
                    ("LM Studio :1234", "http://127.0.0.1:1234"),
                    ("llama-server :8080", "http://127.0.0.1:8080"),
                ] {
                    if ui.small_button(label).clicked() {
                        self.llama_url_edit = url.to_string();
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("[Apply]").clicked() {
                    self.apply_endpoint();
                }
                if ui.button("[Apply and check provider]").clicked() {
                    self.apply_endpoint();
                    self.check_provider();
                }
            });
            if let Some(report) = &self.provider_report {
                ui.add_space(4.0);
                render_provider_report(ui, report);
            }
        });
    }

    /// "What is RSI?" page: the rsi and based_on help tabs, from the same
    /// source as the CLI and MCP help.
    fn render_rsi_and_tsinghua_dossier(&mut self, ui: &mut egui::Ui) {
        ui.heading("What is RSI, and what is rsicraft based on?");
        ui.label(
            "The same text is available as `rsicraft help-tabs --tab rsi` and `--tab based_on`, \
             and over MCP as rsi_help.",
        );
        for id in ["rsi", "based_on"] {
            if let Ok(v) = rsicraft_core::help::query_help(Some(id), None) {
                render_help_tabs(ui, &v);
            }
        }
    }

    fn render_multitab_help_and_tools(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Help");
            ui.separator();
            if ui.button("[Open HTML help]").clicked() {
                self.open_external("docs/help/index.html");
            }
            if ui.button("[Open the guide (docs/GUIDE.md)]").clicked() {
                self.open_external("docs/GUIDE.md");
            }
        });
        ui.label(
            "New here? Read 1. Start here, then 4. Quickstart. When something fails, the Live Run \
             panel names a problem class; 9. Troubleshooting explains each one.",
        );

        let index: Vec<(String, String)> = self
            .help_json
            .get("tabs_available")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|t| {
                        let s =
                            |k: &str| t.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                        (s("id"), s("title"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            let mut pick: Option<String> = None;
            if ui
                .selectable_label(self.selected_help_tab == "all", "All")
                .clicked()
            {
                pick = Some("all".to_string());
            }
            for (id, title) in &index {
                if ui
                    .selectable_label(&self.selected_help_tab == id, title)
                    .clicked()
                {
                    pick = Some(id.clone());
                }
            }
            if let Some(p) = pick {
                self.selected_help_tab = p;
                self.refresh_help();
            }
            ui.separator();
            ui.label("Search:");
            if ui
                .add(egui::TextEdit::singleline(&mut self.help_search).desired_width(160.0))
                .changed()
            {
                self.refresh_help();
            }
        });

        if self
            .help_json
            .get("matched_tabs_count")
            .and_then(Value::as_u64)
            == Some(0)
        {
            ui.label("Nothing matches this search.");
        }
        render_help_tabs(ui, &self.help_json);
    }
}

/// Render the `tabs` of an `rsi_help` result as cards.
fn render_help_tabs(ui: &mut egui::Ui, help: &Value) {
    let Some(tabs) = help.get("tabs").and_then(Value::as_array) else {
        return;
    };
    for t in tabs {
        let title = t.get("title").and_then(Value::as_str).unwrap_or("");
        let badge = t.get("badge").and_then(Value::as_str).unwrap_or("");
        let summary = t.get("summary").and_then(Value::as_str).unwrap_or("");
        accent_card(ui, ACCENT_CYAN, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(title);
                ui.label(RichText::new(format!("[{badge}]")).color(ACCENT_EMERALD));
            });
            ui.add(egui::Label::new(RichText::new(summary).italics()).wrap());
            if let Some(sections) = t.get("sections").and_then(Value::as_array) {
                for s in sections {
                    ui.separator();
                    let h = s.get("heading").and_then(Value::as_str).unwrap_or("");
                    let b = s.get("body").and_then(Value::as_str).unwrap_or("");
                    ui.label(RichText::new(h).color(ACCENT_AMBER).strong());
                    ui.add(egui::Label::new(b).wrap());
                    if let Some(code) = s.get("code_example").and_then(Value::as_str) {
                        ui.code(code);
                    }
                }
            }
        });
        ui.add_space(6.0);
    }
}

/// A step or provider failure, wrapped and readable: what happened, and the
/// fix on its own line. Text format from the core: "[llm:<class>] what | fix: how".
fn render_failure_box(ui: &mut egui::Ui, class: &str, text: &str) {
    let (what, fix) = match text.split_once(" | fix: ") {
        Some((w, f)) => (w, Some(f)),
        None => (text, None),
    };
    let what = what
        .strip_prefix(&format!("[llm:{class}] "))
        .unwrap_or(what);
    egui::Frame::new()
        .stroke(Stroke::new(1.5, ACCENT_ROSE))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(format!("Problem: {class}"))
                    .color(ACCENT_ROSE)
                    .strong(),
            );
            ui.add(egui::Label::new(what).wrap());
            if let Some(fix) = fix {
                ui.label(RichText::new("Fix:").color(ACCENT_EMERALD).strong());
                ui.add(egui::Label::new(fix).wrap());
            }
            ui.label(RichText::new("Help page, tab Troubleshooting, explains every class.").weak());
        });
}

/// Human view of an `rsi_llm_probe` report.
fn render_provider_report(ui: &mut egui::Ui, r: &Value) {
    let ok = r.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let s = |k: &str| r.get(k).and_then(Value::as_str).unwrap_or("-").to_string();
    ui.horizontal_wrapped(|ui| {
        if ok {
            ui.label(RichText::new("Provider OK").color(ACCENT_EMERALD).strong());
        } else {
            ui.label(
                RichText::new("Provider NOT usable")
                    .color(ACCENT_ROSE)
                    .strong(),
            );
        }
        ui.label(format!(
            "server: {} | address: {} | model: {} | latency: {} ms",
            s("provider"),
            s("base_url"),
            s("configured_model"),
            r.get("latency_ms").and_then(Value::as_u64).unwrap_or(0)
        ));
    });
    if let Some(f) = r.get("failure").filter(|f| f.is_object()) {
        let class = f.get("kind").and_then(Value::as_str).unwrap_or("error");
        let msg = f.get("message").and_then(Value::as_str).unwrap_or("");
        let fix = f.get("remedy").and_then(Value::as_str).unwrap_or("");
        render_failure_box(ui, class, &format!("{msg} | fix: {fix}"));
    }
    if let Some(ws) = r.get("warnings").and_then(Value::as_array) {
        for w in ws.iter().filter_map(Value::as_str) {
            ui.add(
                egui::Label::new(RichText::new(format!("Warning: {w}")).color(ACCENT_AMBER)).wrap(),
            );
        }
    }
    if let Some(models) = r.get("available_models").and_then(Value::as_array) {
        if !models.is_empty() {
            let names: Vec<&str> = models.iter().filter_map(Value::as_str).collect();
            ui.collapsing(format!("Models served ({})", names.len()), |ui| {
                ui.add(egui::Label::new(names.join(", ")).wrap());
            });
        }
    }
}

impl eframe::App for RsiCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.style_initialized {
            configure_typography(ui.ctx());
            ui.ctx().set_visuals(harsh_visuals());
            self.style_initialized = true;
        }
        handle_zoom(ui.ctx());
        if self.theme_dirty {
            ui.ctx().set_visuals(if self.dark_mode {
                harsh_visuals()
            } else {
                crisp_light_visuals()
            });
            self.theme_dirty = false;
        }
        if self.zoom_reset {
            ui.ctx().set_zoom_factor(1.0);
            self.zoom_reset = false;
        }
        if let Some(text) = self.pending_copy.take() {
            ui.ctx().copy_text(text);
        }
        if self.quit_requested {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            self.quit_requested = false;
        }
        self.render_menu_bar(ui);
        self.apply_file_picks();
        self.render_active_dialog(ui.ctx());
        let typing = ui.ctx().egui_wants_keyboard_input();
        let mut hotkey: Option<&'static str> = None;
        ui.input(|i| {
            if i.key_pressed(egui::Key::F1) {
                hotkey = Some("shortcuts");
            } else if i.modifiers.command && i.key_pressed(egui::Key::K) {
                hotkey = Some("palette");
            } else if !typing && i.modifiers.command && i.key_pressed(egui::Key::N) {
                hotkey = Some("new_task");
            } else if !typing && i.modifiers.command && i.key_pressed(egui::Key::O) {
                hotkey = Some("open_db");
            } else if !typing
                && i.modifiers.command
                && i.modifiers.shift
                && i.key_pressed(egui::Key::S)
            {
                hotkey = Some("save_as");
            } else if !typing && i.modifiers.command && i.key_pressed(egui::Key::S) {
                hotkey = Some("save");
            } else if !typing && i.modifiers.command && i.key_pressed(egui::Key::Z) {
                hotkey = Some("undo");
            } else if !typing && i.modifiers.command && i.key_pressed(egui::Key::Y) {
                hotkey = Some("redo");
            } else if !typing && i.modifiers.command && i.key_pressed(egui::Key::E) {
                hotkey = Some("export");
            } else if !typing && i.modifiers.command && i.key_pressed(egui::Key::Q) {
                hotkey = Some("quit");
            } else if !typing && i.modifiers.command && i.key_pressed(egui::Key::Num1) {
                hotkey = Some("zoom_reset");
            }
        });
        if let Some(id) = hotkey {
            if let Err(e) = self.menu_action(id) {
                self.status_banner = e;
            }
        }
        if self.repaint_ctx.is_none() {
            self.repaint_ctx = Some(ui.ctx().clone());
        }
        self.poll_job();
        if self.job.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        self.anim_tick = self.anim_tick.wrapping_add(1);
        if self.gpu_auto_spin || self.gpu_plasma_fx || self.rsi_3d_auto_spin {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }

        // Top Header Bar
        ui.horizontal_wrapped(|ui| {
            paint_status_dot(ui, ACCENT_EMERALD);
            ui.heading(
                RichText::new(format!("RSICraft v{VERSION} Sovereign Studio"))
                    .color(ACCENT_CYAN)
                    .strong(),
            );
            ui.separator();
            let _ = theme_toggle_button(ui, &mut self.dark_mode);
            let zoom_pct = (ui.ctx().zoom_factor() * 100.0).round() as u32;
            if ui
                .small_button(format!("{zoom_pct}%"))
                .on_hover_text("Click to reset zoom to 100% (or use Ctrl+MouseWheel)")
                .clicked()
            {
                ui.ctx().set_zoom_factor(1.0);
            }
            ui.separator();
            if ui.button("[PDF Brochure]").clicked() {
                self.open_external("docs/brochure/rsicraft-brochure.pdf");
            }
            if ui.button("[EPUB Handbook]").clicked() {
                self.open_external("docs/brochure/rsicraft-brochure.epub");
            }
            if ui
                .button(if self.show_quick_help_modal {
                    "[x Hide intro]"
                } else {
                    "[? New here]"
                })
                .clicked()
            {
                self.show_quick_help_modal = !self.show_quick_help_modal;
            }
            ui.separator();
            ui.label(
                RichText::new(&self.status_banner)
                    .color(ACCENT_EMERALD)
                    .monospace(),
            );
        });

        ui.add_space(4.0);
        self.render_kpi_ribbon(ui);
        ui.separator();

        // Navigation Bar
        ui.horizontal_wrapped(|ui| {
            for p in StudioPage::all() {
                if ui.selectable_label(self.page == p, p.label()).clicked() {
                    self.page = p;
                }
            }
        });

        if self.show_quick_help_modal {
            ui.separator();
            accent_card(ui, ACCENT_PURPLE, |ui| {
                ui.heading("New here?");
                ui.add(egui::Label::new("rsicraft lets a language model on your machine write, test and improve small programs in a loop, and records every attempt with its parents. The model itself does not change.").wrap());
                ui.add(egui::Label::new("1. Try it without a model: provider demo, sandbox demo, then Run campaign on this page. Scores are invented in demo mode.").wrap());
                ui.add(egui::Label::new("2. Connect a real model on the LLM page (Ollama, LM Studio, llama-server), press Apply and check provider, then choose provider llama and sandbox local or docker.").wrap());
                ui.add(egui::Label::new("3. Read the Help page: Start here, What is RSI?, Quickstart. Problems appear in the Live Run panel with their fix.").wrap());
                if ui.button("[Open Help: Start here]").clicked() {
                    self.page = StudioPage::MultitabHelpAndTools;
                    self.selected_help_tab = "start".to_string();
                    self.refresh_help();
                }
            });
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| match self.page {
            StudioPage::EvolutionaryStudio => self.render_evolutionary_studio(ui),
            StudioPage::LineageDagCanvas => self.render_lineage_dag_canvas(ui),
            StudioPage::ExperienceCards => self.render_experience_cards_page(ui),
            StudioPage::TaskAndSandboxLab => self.render_task_and_sandbox_lab(ui),
            StudioPage::Goliath4090AndLlm => self.render_goliath_4090_and_llm(ui),
            StudioPage::RsiAndTsinghuaDossier => self.render_rsi_and_tsinghua_dossier(ui),
            StudioPage::MultitabHelpAndTools => self.render_multitab_help_and_tools(ui),
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--smoke") {
        let Ok(mut app) = RsiCraftApp::new(None) else {
            eprintln!("smoke: failed to initialize RsiCraftApp");
            std::process::exit(1);
        };
        let ok = app.exec(Command::Status).is_ok()
            && app.exec(Command::HardwareProfile).is_ok()
            && app
                .exec(Command::Help {
                    tab: Some("based_on".into()),
                    search: None,
                })
                .is_ok()
            && app.view_mode == ViewMode::Cards
            && tools().len() == 26
            && app.menu_action("frontier").is_ok()
            && app.menu_action("about").is_ok()
            && app.active_dialog == Some(StudioDialog::About);
        if ok {
            println!(
                "smoke: ok (rsicraft-app {VERSION}, default_view=cards, tools=26, help_tabs={}, menu=8)",
                rsicraft_core::help::tab_ids().len()
            );
            std::process::exit(0);
        }
        eprintln!("smoke: failed");
        std::process::exit(1);
    }

    let db_path = std::env::var("RSICRAFT_DB").ok().map(PathBuf::from);
    let Ok(app) = RsiCraftApp::new(db_path.as_deref()) else {
        eprintln!("rsicraft-app: failed to open database");
        std::process::exit(1);
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RSICraft - Sovereign Recursive Self-Improvement Studio (RTX 4090 / Frontis-MA1 / OpenRSI)")
            .with_inner_size([1360.0, 860.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "rsicraft-app",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_dispatch_matches_core_and_defaults_to_cards() {
        let mut app = RsiCraftApp::new(None).unwrap();
        assert_eq!(app.view_mode, ViewMode::Cards);
        let st = app.exec(Command::Status).unwrap();
        assert_eq!(st["cards_count"], 4);
        assert_eq!(tools().len(), 26);
    }

    #[test]
    fn background_campaign_runs_off_the_ui_thread_and_commits_each_step() {
        let mut app = RsiCraftApp::new(None).unwrap();
        app.campaign_steps = 3;
        app.start_campaign();
        assert!(app.is_busy());

        // A second run cannot start while one is in flight.
        app.trigger_step(SearchOperator::Draft);
        assert!(app.status_banner.contains("already in progress"));

        assert!(app.wait_for_job(Duration::from_secs(20)));
        assert_eq!(app.run_feed.len(), 3);
        assert_eq!(app.exec(Command::Status).unwrap()["cards_count"], 7);
        assert!(app.status_banner.contains("finished"));

        // Stop request ends a campaign after the step in flight.
        app.campaign_steps = 5;
        app.start_campaign();
        app.request_stop();
        assert!(app.wait_for_job(Duration::from_secs(20)));
        assert_eq!(app.run_feed.len(), 4);
        assert!(app.status_banner.contains("stopped"));

        // Single step and lab evaluation use the same path.
        app.trigger_step(SearchOperator::Improve);
        assert!(app.wait_for_job(Duration::from_secs(20)));
        app.start_evaluate();
        assert!(app.wait_for_job(Duration::from_secs(20)));
        assert!(app.lab_last_eval.is_some());
        assert_eq!(app.exec(Command::Status).unwrap()["cards_count"], 10);

        // A step that cannot be planned reports the reason and starts nothing.
        app.provider_pick = "bogus".to_string();
        app.trigger_step(SearchOperator::Draft);
        assert!(!app.is_busy());
        assert!(app.status_banner.contains("invalid provider"));
    }

    #[test]
    fn menu_catalog_reaches_dispatch_or_a_dialog() {
        let mut app = RsiCraftApp::new(None).unwrap();
        for (id, _label, _shortcut) in MENU_CATALOG {
            if matches!(
                *id,
                "probe" | "campaign" | "step_improve" | "step_debug" | "step_crossover"
            ) {
                continue;
            }
            let err = app.menu_action(id).err();
            if let Some(msg) = err {
                assert!(
                    !msg.contains("unknown menu action"),
                    "{id} is not wired: {msg}"
                );
            }
        }
        assert_eq!(app.active_dialog, Some(StudioDialog::About));
        assert!(app.menu_action("frontier").unwrap()["frontier"].is_array());
    }

    #[test]
    fn undo_and_save_as_roundtrip_the_store() {
        let mut app = RsiCraftApp::new(None).unwrap();
        let before = app.exec(Command::Status).unwrap()["tasks_count"]
            .as_u64()
            .unwrap();
        app.undoable(
            "task",
            Command::TaskUpsert {
                task_id: "undo-me".into(),
                name: "Undo me".into(),
                description: "temporary".into(),
                evaluator_script: "print('RSI_METRIC_JSON={\"status\":\"SUCCESS\",\"score\":1}')"
                    .into(),
            },
        )
        .unwrap();
        let mid = app.exec(Command::Status).unwrap()["tasks_count"]
            .as_u64()
            .unwrap();
        assert!(mid > before);
        app.menu_action("undo").unwrap();
        assert_eq!(
            app.exec(Command::Status).unwrap()["tasks_count"].as_u64(),
            Some(before)
        );
        app.menu_action("redo").unwrap();
        assert_eq!(
            app.exec(Command::Status).unwrap()["tasks_count"].as_u64(),
            Some(mid)
        );

        let dir = std::env::temp_dir().join(format!("rsicraft-app-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("studio.db");
        app.save_store_as(path.clone()).unwrap();
        assert_eq!(app.ctx.reg.db_path(), Some(path.as_path()));
        app.open_store_at(path.clone()).unwrap();
        assert_eq!(
            app.exec(Command::Status).unwrap()["tasks_count"].as_u64(),
            Some(mid)
        );
        app.menu_action("undo").unwrap();
        assert_eq!(app.ctx.reg.db_path(), Some(path.as_path()));
        app.menu_action("undo").unwrap();
        assert!(app.ctx.reg.db_path().is_none());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn glyph_allowlist_no_tofu_chars() {
        let src = include_str!("main.rs");
        let allowed_non_ascii = ['°', '×', '·', '…', '-'];
        for (line_no, line) in src.lines().enumerate() {
            for ch in line.chars() {
                assert!(
                    ch.is_ascii() || allowed_non_ascii.contains(&ch),
                    "Forbidden non-ASCII glyph '{ch}' (U+{:04X}) on line {} would render as tofu in egui 0.37",
                    ch as u32,
                    line_no + 1
                );
            }
        }
    }
}
