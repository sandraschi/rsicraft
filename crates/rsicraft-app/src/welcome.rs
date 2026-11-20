//! First-time user Welcome Screen ("3-3-1" studio pattern):
//! 1-line identity, 3 visual 1-click starter cards, 3 readiness pills,
//! persisted startup preference, and vector RSICraft emblem.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use rsicraft_core::VERSION;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeAction {
    None,
    SeedDemo,
    InspectLineage,
    OpenHelpCenter,
    EnterWorkspace,
}

pub struct WelcomeState {
    pub open: bool,
    pub show_on_startup: bool,
}

impl Default for WelcomeState {
    fn default() -> Self {
        let show = should_show_on_startup();
        Self {
            open: show,
            show_on_startup: show,
        }
    }
}

impl WelcomeState {
    pub fn set_show_on_startup(&mut self, show: bool) {
        self.show_on_startup = show;
        save_startup_pref(show);
    }

    pub fn draw(&mut self, ctx: &egui::Context) -> WelcomeAction {
        if !self.open {
            return WelcomeAction::None;
        }
        let mut action = WelcomeAction::None;

        // Softly dimmed backdrop
        let screen = ctx.content_rect();
        egui::Area::new(egui::Id::unique("welcome_backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_black_alpha(170));
            });

        egui::Window::new("Welcome to RSICraft")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(760.0, 470.0))
            .frame(
                egui::Frame::new()
                    .corner_radius(10.0)
                    .fill(Color32::from_rgb(10, 15, 26))
                    .stroke(Stroke::new(1.2, Color32::from_rgb(54, 80, 122)))
                    .inner_margin(egui::Margin::same(28)),
            )
            .show(ctx, |ui| {
                // 1. Hero Header with Vector Emblem
                ui.horizontal(|ui| {
                    draw_emblem(ui, Vec2::new(64.0, 64.0));
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(format!("RSICraft Studio  v{VERSION}"))
                                .size(24.0)
                                .strong()
                                .color(Color32::from_rgb(242, 246, 252)),
                        );
                        ui.label(
                            egui::RichText::new(
                                "Local-first OpenRSI evolutionary program search, BLAKE3 CAS, and Pareto frontier studio.",
                            )
                            .size(14.5)
                            .color(Color32::from_rgb(176, 198, 224)),
                        );
                    });
                });

                ui.add_space(18.0);
                ui.separator();
                ui.add_space(14.0);

                // 2. Three 1-click Visual Starter Cards
                ui.columns(3, |cols| {
                    action = starter_card(
                        &mut cols[0],
                        "1. Seed Demo Tasks & Frontier",
                        "Populates the SQLite + BLAKE3 CAS store with sample OpenRSI tasks, experience cards, and a Pareto frontier.",
                        "Seed Demo",
                        Color32::from_rgb(52, 211, 153),
                        WelcomeAction::SeedDemo,
                    )
                    .or_else_action(action);

                    action = starter_card(
                        &mut cols[1],
                        "2. Inspect Lineage DAG / Tasks",
                        "Opens the GPU-painted Bezier Lineage DAG canvas to explore parent-child mutations, crossovers, and scores.",
                        "Open Lineage DAG",
                        Color32::from_rgb(251, 191, 36),
                        WelcomeAction::InspectLineage,
                    )
                    .or_else_action(action);

                    action = starter_card(
                        &mut cols[2],
                        "3. Guided Quickstart / Help Center",
                        "Step-by-step OpenRSI workflow, local LLM setup, safety sandboxing, bundled skills, and CLI/MCP guides.",
                        "Open Help Center",
                        Color32::from_rgb(56, 189, 248),
                        WelcomeAction::OpenHelpCenter,
                    )
                    .or_else_action(action);
                });

                ui.add_space(18.0);

                // 3. Three Compact Readiness Pills + Keycap Hints
                ui.horizontal_wrapped(|ui| {
                    pill(ui, "BLAKE3 CAS: Ready", Color32::from_rgb(52, 211, 153));
                    pill(ui, "2 Bundled Skills", Color32::from_rgb(251, 191, 36));
                    pill(ui, "AI Chat: Ctrl+J", Color32::from_rgb(56, 189, 248));
                    ui.separator();
                    ui.small("Ctrl+K Palette  |  F1 Shortcuts  |  Ctrl+J AI Chat");
                });

                ui.add_space(18.0);
                ui.separator();
                ui.add_space(10.0);

                // 4. Footer: Startup Preference + Enter Workspace
                ui.horizontal(|ui| {
                    let mut show = self.show_on_startup;
                    if ui.checkbox(&mut show, "Show welcome screen on startup").changed() {
                        self.set_show_on_startup(show);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(
                                egui::RichText::new("Enter Workspace ->")
                                    .size(14.5)
                                    .strong(),
                            )
                            .clicked()
                        {
                            action = WelcomeAction::EnterWorkspace;
                        }
                    });
                });
            });

        if action != WelcomeAction::None {
            self.open = false;
        }
        action
    }
}

trait ActionOr {
    fn or_else_action(self, prev: WelcomeAction) -> WelcomeAction;
}

impl ActionOr for WelcomeAction {
    fn or_else_action(self, prev: WelcomeAction) -> WelcomeAction {
        if self != WelcomeAction::None {
            self
        } else {
            prev
        }
    }
}

fn starter_card(
    ui: &mut egui::Ui,
    title: &str,
    desc: &str,
    btn: &str,
    accent: Color32,
    on_click: WelcomeAction,
) -> WelcomeAction {
    let mut act = WelcomeAction::None;
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(16, 23, 38))
        .stroke(Stroke::new(1.0, Color32::from_rgb(44, 64, 96)))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_height(175.0);
            ui.label(egui::RichText::new(title).size(15.5).strong().color(accent));
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(desc)
                    .size(13.0)
                    .color(Color32::from_rgb(190, 206, 228)),
            );
            ui.add_space(12.0);
            if ui.button(btn).clicked() {
                act = on_click;
            }
        });
    act
}

fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(Color32::from_rgb(18, 28, 46))
        .stroke(Stroke::new(1.0, color))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(12.0).color(color));
        });
}

fn draw_emblem(ui: &mut egui::Ui, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter_at(rect);
    let cyan = Color32::from_rgb(56, 189, 248);
    let emerald = Color32::from_rgb(52, 211, 153);
    let amber = Color32::from_rgb(251, 191, 36);

    p.rect_filled(rect, 10.0, Color32::from_rgb(15, 23, 42));
    p.rect_stroke(rect, 10.0, Stroke::new(1.5, cyan), egui::StrokeKind::Inside);

    // Evolutionary DAG nodes + Pareto frontier curve
    let r = rect.shrink(10.0);
    let root = Pos2::new(r.left() + 4.0, r.center().y + 4.0);
    let mid_top = Pos2::new(r.center().x - 4.0, r.top() + 10.0);
    let mid_bot = Pos2::new(r.center().x - 2.0, r.bottom() - 8.0);
    let leaf_top = Pos2::new(r.right() - 4.0, r.top() + 6.0);
    let leaf_mid = Pos2::new(r.right() - 4.0, r.center().y - 2.0);

    // DAG edges
    p.line_segment([root, mid_top], Stroke::new(1.8, cyan));
    p.line_segment([root, mid_bot], Stroke::new(1.6, amber));
    p.line_segment([mid_top, leaf_top], Stroke::new(1.8, emerald));
    p.line_segment([mid_top, leaf_mid], Stroke::new(1.6, cyan));
    p.line_segment([mid_bot, leaf_mid], Stroke::new(1.6, amber));

    // Pareto frontier curve sweeping across the right/upper-right
    p.add(egui::Shape::line(
        vec![
            Pos2::new(r.left() + 8.0, r.bottom() - 2.0),
            Pos2::new(r.center().x - 6.0, r.center().y + 4.0),
            Pos2::new(r.center().x + 6.0, r.top() + 16.0),
            Pos2::new(r.right() - 2.0, r.top() + 5.0),
        ],
        Stroke::new(2.0, emerald),
    ));

    // DAG nodes
    p.circle_filled(root, 3.5, amber);
    p.circle_filled(mid_top, 3.5, cyan);
    p.circle_filled(mid_bot, 3.0, amber);
    p.circle_filled(leaf_top, 4.0, emerald);
    p.circle_filled(leaf_mid, 3.5, cyan);
}

fn pref_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .map(|root| {
            PathBuf::from(root)
                .join("rsicraft")
                .join("welcome_hide.flag")
        })
}

fn should_show_on_startup() -> bool {
    let Some(path) = pref_path() else {
        return true;
    };
    !path.exists()
}

fn save_startup_pref(show: bool) {
    let Some(path) = pref_path() else {
        return;
    };
    if show {
        let _ = std::fs::remove_file(path);
    } else {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, b"hide");
    }
}
