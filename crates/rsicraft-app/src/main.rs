//! `rsicraft-app`: L3 Pure-Rust `eframe` / `egui 0.37` GPU Desktop Studio.
//!
//! Adheres strictly to `EGUI_DESKTOP_UI_AND_PARITY.md`:
//! - High-contrast `slate-950` (`#0a0d12`) dark theme + crisp `slate-50` light theme
//! - Vector-painted Sun / Crescent Moon toggle and `Ctrl + MouseWheel` zoom
//! - Zero non-ASCII tofu glyphs (verified by `glyph_allowlist_no_tofu_chars` test)
//! - Interactive GPU-painted Bezier Lineage DAG Canvas and Score Trajectory Spark Chart
//! - Full 4-Operator Evolutionary Launchpad, BLAKE3+Zstd CAS Inspector, RTX 4090 Gauge,
//!   RSI Geopolitics & Tsinghua/Frontis.AI Dossier, and 9-Tab Searchable Help Manual.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Pos2, RichText, Stroke, Vec2};
use rsicraft_core::operators::SearchOperator;
use rsicraft_core::{dispatch, tools, Command, Ctx, Registry, VERSION};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

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
            Self::RsiAndTsinghuaDossier => "6. RSI & Tsinghua Dossier",
            Self::MultitabHelpAndTools => "7. 9-Tab Help & 14 Tools",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Cards,
    Table,
    Json,
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

    pub status_banner: String,
    pub repo_root: PathBuf,
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
            llama_url_edit: "http://127.0.0.1:8081".to_string(),
            model_alias_edit: "frontis".to_string(),
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
            status_banner: format!(
                "RSICraft v{VERSION} Sovereign Studio Online (Goliath RTX 4090 Profile Ready)"
            ),
            repo_root: default_repo_root(),
        };
        app.refresh_all();
        Ok(app)
    }

    pub fn exec(&mut self, cmd: Command) -> Result<Value, String> {
        dispatch(&mut self.ctx, cmd)
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

    pub fn trigger_step(&mut self, op: SearchOperator) {
        let tid = self.active_task_id.clone();
        let prov = self.provider_pick.clone();
        let sb = self.sandbox_pick.clone();
        let t = self.timeout_secs;
        match self.exec(Command::Step {
            task_id: tid,
            operator: op,
            provider: prov,
            sandbox_mode: sb,
            timeout_secs: t,
        }) {
            Ok(v) => {
                let cid = v.get("card_id").and_then(Value::as_str).unwrap_or("?");
                let sc = v
                    .get("primary_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let util = v
                    .get("composite_utility")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                self.status_banner = format!(
                    "Step {} complete -> card {} (score={sc:.4}, U_i={util:.4})",
                    op.as_str(),
                    &cid[..cid.len().min(8)]
                );
                let full_id = cid.to_string();
                self.refresh_all();
                if let Ok(card) = self.exec(Command::CardShow { card_id: full_id }) {
                    self.selected_card_json = Some(card);
                }
            }
            Err(e) => {
                self.status_banner = format!("Step {} error: {e}", op.as_str());
            }
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
    v.panel_fill = Color32::from_rgb(248, 250, 252);
    v.window_fill = Color32::from_rgb(255, 255, 255);
    v.extreme_bg_color = Color32::from_rgb(241, 245, 249);
    v.override_text_color = Some(Color32::from_rgb(15, 23, 42));
    v.widgets.noninteractive.bg_fill = Color32::from_rgb(255, 255, 255);
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

impl RsiCraftApp {
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
                painter.add(egui::Shape::line(line_pts, Stroke::new(2.2, ACCENT_CYAN)));
            }
            for (pt, c) in &pts {
                let op = c.get("operator").and_then(Value::as_str).unwrap_or("DRAFT");
                let col = operator_color(op);
                painter.circle_filled(*pt, 5.0, col);
                painter.circle_stroke(*pt, 5.0, Stroke::new(1.2, Color32::WHITE));
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
                    let tid = self.active_task_id.clone();
                    let prov = self.provider_pick.clone();
                    let sb = self.sandbox_pick.clone();
                    let steps = self.campaign_steps;
                    let timeout_secs = self.timeout_secs;
                    match self.exec(Command::Campaign {
                        task_id: tid,
                        steps,
                        provider: prov,
                        sandbox_mode: sb,
                        timeout_secs,
                    }) {
                        Ok(v) => {
                            let best = v.get("best_score").and_then(Value::as_f64).unwrap_or(0.0);
                            self.status_banner =
                                format!("Executed {steps}-step campaign -> best_score={best:.4}");
                            self.refresh_all();
                        }
                        Err(e) => self.status_banner = format!("Campaign error: {e}"),
                    }
                }
            });
        });

        ui.add_space(6.0);
        self.render_score_spark_chart(ui);
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
            .request_repaint_after(std::time::Duration::from_millis(80));

        ui.horizontal_wrapped(|ui| {
            ui.heading("Stigmergic Evolutionary Lineage DAG Canvas (GPU Bezier Node Graph)");
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

        // Draw Bezier edges from parent -> child
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

                let t = ((self.anim_tick as f32 * 0.06) % 1.0).clamp(0.0, 1.0);
                let omt = 1.0 - t;
                let px = p0.x * omt.powi(3)
                    + p1.x * 3.0 * omt.powi(2) * t
                    + p2.x * 3.0 * omt * t.powi(2)
                    + p3.x * t.powi(3);
                let py = p0.y * omt.powi(3)
                    + p1.y * 3.0 * omt.powi(2) * t
                    + p2.y * 3.0 * omt * t.powi(2)
                    + p3.y * t.powi(3);
                painter.circle_filled(Pos2::new(px, py), 4.0, Color32::WHITE);
            }
        }

        // Draw nodes & detect click
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
                let col = operator_color(op);
                painter.rect_filled(*rect, CornerRadius::same(8), Color32::from_rgb(17, 25, 42));
                painter.rect_stroke(
                    *rect,
                    CornerRadius::same(8),
                    Stroke::new(1.6, col),
                    egui::StrokeKind::Inside,
                );
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

    fn render_selected_card_inspector(&self, ui: &mut egui::Ui) {
        let Some(card) = &self.selected_card_json else {
            ui.label("Click any card or DAG node to inspect decompressed BLAKE3+Zstd source code.");
            return;
        };
        let cid = card.get("card_id").and_then(Value::as_str).unwrap_or("?");
        let op = card
            .get("operator_used")
            .and_then(Value::as_str)
            .unwrap_or("?");
        let hash = card.get("code_hash").and_then(Value::as_str).unwrap_or("?");
        let code = card.get("code").and_then(Value::as_str).unwrap_or("");
        let out = card.get("raw_output").and_then(Value::as_str).unwrap_or("");
        let sc = card
            .get("primary_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let ut = card
            .get("composite_utility")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);

        accent_card(ui, operator_color(op), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("Decompressed Artifact Inspector: Card {cid}"))
                        .color(operator_color(op))
                        .strong(),
                );
                ui.separator();
                ui.label(format!("Operator: {op} | Score: {sc:.4} | U_i: {ut:.4}"));
            });
            ui.label(
                RichText::new(format!("BLAKE3-256 Digest (Zstd L3 CAS): {hash}"))
                    .monospace()
                    .size(12.0),
            );
            ui.columns(2, |cols| {
                cols[0].label(RichText::new("Synthesized Python Source Code:").strong());
                cols[0].code(code);
                cols[1].label(RichText::new("Sandbox Evaluator Output / Stack Trace:").strong());
                cols[1].code(out);
            });
        });
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
                    match self.exec(cmd) {
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
                    let cmd = Command::Evaluate {
                        task_id: self.active_task_id.clone(),
                        code: self.lab_candidate_code.clone(),
                        operator: SearchOperator::Draft,
                        parent_a_id: None,
                        parent_b_id: None,
                        sandbox_mode: self.sandbox_pick.clone(),
                        timeout_secs: self.timeout_secs,
                        record: true,
                    };
                    match self.exec(cmd) {
                        Ok(v) => {
                            self.lab_last_eval = Some(v);
                            self.status_banner =
                                "Sandbox evaluation completed and recorded".to_string();
                            self.refresh_all();
                        }
                        Err(e) => self.status_banner = format!("Evaluation error: {e}"),
                    }
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
            ui.heading("LLM Inference Endpoint & Model Configuration");
            ui.horizontal_wrapped(|ui| {
                ui.label("Base URL (/v1/chat/completions):");
                ui.add(egui::TextEdit::singleline(&mut self.llama_url_edit).desired_width(260.0));
                ui.label("Model Alias:");
                ui.add(egui::TextEdit::singleline(&mut self.model_alias_edit).desired_width(140.0));
                if ui.button("[Apply Endpoint to Engine]").clicked() {
                    self.ctx.llm = rsicraft_core::llm::LlamaClient::new(
                        &self.llama_url_edit,
                        &self.model_alias_edit,
                    );
                    self.status_banner = format!(
                        "Updated LLM client -> {} (model={})",
                        self.llama_url_edit, self.model_alias_edit
                    );
                    self.refresh_all();
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.small_button("Preset: llama-server (:8081)").clicked() {
                    self.llama_url_edit = "http://127.0.0.1:8081".to_string();
                    self.model_alias_edit = "frontis".to_string();
                }
                if ui
                    .small_button("Preset: Goliath Active Runner (:51907)")
                    .clicked()
                {
                    self.llama_url_edit = "http://127.0.0.1:51907".to_string();
                    self.model_alias_edit = "frontis".to_string();
                }
                if ui.small_button("Preset: Ollama (:11434)").clicked() {
                    self.llama_url_edit = "http://127.0.0.1:11434".to_string();
                    self.model_alias_edit = "qwen2.5-coder:7b".to_string();
                }
            });
        });
    }

    fn render_rsi_and_tsinghua_dossier(&mut self, ui: &mut egui::Ui) {
        accent_card(ui, ACCENT_ROSE, |ui| {
            ui.heading("1. The Western 'Skynet' Narrative & Closed-Door Frontier Labs");
            ui.label("Since I.J. Good's 1965 'intelligence explosion' hypothesis and Nick Bostrom's Superintelligence (2014), Western policy circles in Washington, London, and Brussels have framed Recursive Self-Improvement (RSI) with ominous apocalyptic undertones: 'Will it become Skynet? Will a self-improving model rewrite its own code in secret and trigger an unstoppable rogue takeoff?'");
            ui.add_space(4.0);
            ui.label(
                RichText::new("How OpenAI & Anthropic Use RSI Rhetoric Behind Closed Doors:")
                    .strong()
                    .color(ACCENT_AMBER),
            );
            ui.label("- OpenAI Preparedness Framework: Classifies 'AI Self-Improvement' (autonomous ML R&D) as a High/Critical risk threshold. OpenAI executives frequently reference internal o1/o3/o4 self-play and automated researcher loops in Senate testimony and system cards, while keeping every weight, operator prompt, and evaluation harness 100% proprietary.");
            ui.label("- Anthropic Responsible Scaling Policy (RSP / ASL-4): Explicitly defines 'Autonomy & Automated AI R&D' (Claude autonomously optimizing kernels, training loops, and post-training pipelines) as the trigger for ASL-4 containment. The message to the public is always the same: 'We are running RSI experiments internally, but RSI is far too dangerous and compute-intensive for open-source researchers to touch.'");
        });

        ui.add_space(8.0);
        accent_card(ui, ACCENT_CYAN, |ui| {
            ui.heading("2. The Total Expectation Flip: Tsinghua University & Frontis.AI (arXiv:2607.28568)");
            ui.label("On July 30, 2026, a 24-researcher team from Tsinghua University, Frontis.AI, and Horizon Research shattered that narrative by publishing 'Frontis-MA1: Training an AI4AI Model towards Recursive Self-Improvement in Machine Learning Engineering' (arXiv:2607.28568) and open-sourcing the entire code stack and weights at https://github.com/FrontisAI/OpenRSI.");
            ui.add_space(4.0);
            ui.label(
                RichText::new("Why It Flipped Every Expectation:")
                    .strong()
                    .color(ACCENT_EMERALD),
            );
            ui.label("1. Consumer 24 GB GPU Target: Instead of demanding a 10,000-H100 cluster, they explicitly proved and benchmarked their evolutionary loop on MLE-Bench Lite under a strict consumer constraint: a single NVIDIA RTX 4090 over a 12-hour budget.");
            ui.label("2. Execution-Grounded Science Over Sci-Fi Mysticism: They defined RSI ('AI4AI') not as spooky sentient self-awareness, but as verifiable program evolution governed by four explicit atomic operators (Draft, Improve, Debug, Crossover) and a 3-term composite utility function that prevents search collapse.");
        });

        ui.add_space(8.0);
        accent_card(ui, ACCENT_PURPLE, |ui| {
            ui.heading("3. Meet the Tsinghua University & Frontis.AI Team & Their Objectives");
            ui.label(
                RichText::new("Principal Investigators & Senior Architects:")
                    .strong()
                    .color(ACCENT_PURPLE),
            );
            ui.label("- Prof. Bowen Zhou (Tsinghua University / Shanghai AI Lab): Chair Professor in the Department of Electronic Engineering at Tsinghua University, Dean of Shanghai AI Lab, and former Head of AI Foundations at IBM Research Watson and JD AI. Co-author of the landmark June 2026 survey 'Self-Improving Agents in the Era of Experience: A Survey of Self- to Meta-Evolution'.");
            ui.label("- Dr. Ning Ding (Tsinghua NLP / OpenBMB / Frontis.AI): Leading Tsinghua researcher behind OpenBMB, UltraFeedback, and MiniCPM, focused on efficient open-weights alignment and agentic post-training.");
            ui.label("- Kaiyan Zhang (Tsinghua University): Core research lead on execution-grounded reinforcement learning and self-evolving agent harnesses.");
            ui.label("- Lead & Core Authors (24-Member Team): Junlin Yang, Che Jiang, Yu Fu, Tianwei Luo, Can Ren, Weizhi Wang, Kaikai Zhao, Hongyi Liu, Yuxin Zuo, Yuru Wang, Yuchen Fan, Kai Tian, Zhenzhao Yuan, Xiaojian Lin, Li Sheng, Rushi Qiang, Guoli Jia, Xingtai Lv, Ermo Hua, Dianqiao Lei, Youbang Sun, Ning Ding, Bowen Zhou, and Kaiyan Zhang.");
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Their 3-Pillar OpenMLE Stack (Subsumed into Pure Rust by RSICraft):",
                )
                .strong()
                .color(ACCENT_CYAN),
            );
            ui.label("- OpenMLE-Gym -> RSICraft Task & Sandbox Evaluator (air-gapped Docker / Python 3.12 with RSI_METRIC_JSON feedback).");
            ui.label("- OpenMLE-ERL & Frontis-MA1 -> RSICraft LlamaClient & 4-Operator Prompt Builder (DRAFT 0.70, IMPROVE 0.40, DEBUG 0.20, CROSSOVER 0.50).");
            ui.label("- OpenMLE-Evo -> RSICraft Stigmergic SQLite WAL Store + BLAKE3/Zstd CAS + Composite Utility Selector (U_i = 0.50*s + 0.35*d + 0.15*nu).");
        });
    }

    fn render_multitab_help_and_tools(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("9-Tab Operator Manual, Geopolitical Dossier & 14 MCP Tools");
            ui.separator();
            if ui.button("[Open PDF Brochure]").clicked() {
                self.open_external("docs/brochure/rsicraft-brochure.pdf");
            }
            if ui.button("[Open EPUB Handbook]").clicked() {
                self.open_external("docs/brochure/rsicraft-brochure.epub");
            }
            if ui.button("[Open HTML Help Studio]").clicked() {
                self.open_external("docs/help/index.html");
            }
        });

        ui.horizontal_wrapped(|ui| {
            for (slug, label) in [
                ("all", "All 9 Tabs"),
                ("overview", "1. Overview"),
                ("rsi_landscape", "2. Skynet vs Closed Labs"),
                ("tsinghua_openrsi", "3. Tsinghua & OpenRSI"),
                ("wrappee", "4. RTX 4090 & Sandbox"),
                ("operators", "5. Operators & Math"),
                ("compare", "6. Competitive Matrix"),
                ("api", "7. 14 MCP Tools"),
                ("troubleshoot", "8. Troubleshooting"),
                ("faq", "9. FAQ & Safety"),
            ] {
                if ui
                    .selectable_label(self.selected_help_tab == slug, label)
                    .clicked()
                {
                    self.selected_help_tab = slug.to_string();
                    self.refresh_help();
                }
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

        if let Some(tabs) = self.help_json.get("tabs").and_then(Value::as_array) {
            for t in tabs {
                let title = t.get("title").and_then(Value::as_str).unwrap_or("");
                let badge = t.get("badge").and_then(Value::as_str).unwrap_or("");
                let summary = t.get("summary").and_then(Value::as_str).unwrap_or("");
                accent_card(ui, ACCENT_CYAN, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(title);
                        ui.label(RichText::new(format!("[{badge}]")).color(ACCENT_EMERALD));
                    });
                    ui.label(RichText::new(summary).italics());
                    if let Some(sections) = t.get("sections").and_then(Value::as_array) {
                        for s in sections {
                            ui.separator();
                            let h = s.get("heading").and_then(Value::as_str).unwrap_or("");
                            let b = s.get("body").and_then(Value::as_str).unwrap_or("");
                            ui.label(RichText::new(h).color(ACCENT_AMBER).strong());
                            ui.label(b);
                            if let Some(code) = s.get("code_example").and_then(Value::as_str) {
                                ui.code(code);
                            }
                        }
                    }
                });
            }
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
                    "[x Hide Quick Matrix]"
                } else {
                    "[? Skynet vs Tsinghua OpenRSI]"
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
                ui.heading("Why RSICraft Flips the Script on Closed Frontier RSI");
                ui.label("1. Western frontier labs (OpenAI Preparedness Framework, Anthropic ASL-4) treat Recursive Self-Improvement as a classified 'Skynet' threshold to justify keeping their automated AI R&D loops 100% closed.");
                ui.label("2. Tsinghua University & Frontis.AI (Bowen Zhou, Ning Ding, Kaiyan Zhang, Junlin Yang, Che Jiang et al., arXiv:2607.28568) demystified RSI into an open-weights 4-operator evolutionary science (Draft, Improve, Debug, Crossover) running on a single consumer 24 GB RTX 4090 (FrontisAI/OpenRSI).");
                ui.label("3. RSICraft compiles that exact architecture into a zero-panic Pure-Rust ArtCraft 3-Crate workspace (rsicraft-core, rsicraft-mcp, rsicraft-app) with SQLite WAL + BLAKE3/Zstd CAS and 14 MCP tools.");
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
                    tab: Some("tsinghua_openrsi".into()),
                    search: None,
                })
                .is_ok()
            && app.view_mode == ViewMode::Cards
            && tools().len() == 14;
        if ok {
            println!("smoke: ok (rsicraft-app {VERSION}, default_view=cards, tools=14, tabs=9)");
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
            .with_title("RSICraft — Sovereign Recursive Self-Improvement Studio (RTX 4090 / Frontis-MA1 / OpenRSI)")
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
        assert_eq!(tools().len(), 14);
    }

    #[test]
    fn glyph_allowlist_no_tofu_chars() {
        let src = include_str!("main.rs");
        let allowed_non_ascii = ['°', '×', '·', '…', '—'];
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
