//! Bundled `SKILL.md` guides embedded at compile time for the Help Center
//! (`Help -> Bundled Skills...`), the `rsicraft_skill_read` MCP tool, and
//! MCP `resources/list` + `resources/read`.

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
pub struct BundledSkill {
    pub name: &'static str,
    pub uri: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

const SKILLS: &[BundledSkill] = &[
    BundledSkill {
        name: "rsicraft-workflow",
        uri: "skill://rsicraft-workflow",
        title: "RSICraft Evolutionary Search Workflow",
        description: "End-to-end evolutionary program search workflow in RSICraft using MCP tools (probe, task setup, campaign, frontier inspection, diffing, and dataset export).",
        content: include_str!("../../../skills/rsicraft-workflow/SKILL.md"),
    },
    BundledSkill {
        name: "rsicraft-ergonomics",
        uri: "skill://rsicraft-ergonomics",
        title: "RSICraft Search Ergonomics & Safety",
        description: "Search tuning, utility hyperparameter balancing, safe sandboxing, and store hygiene in RSICraft.",
        content: include_str!("../../../skills/rsicraft-ergonomics/SKILL.md"),
    },
];

/// Return all bundled skills in display order.
#[must_use]
pub const fn bundled_skills() -> &'static [BundledSkill] {
    SKILLS
}

/// Look up a bundled skill by name or `skill://` URI.
#[must_use]
pub fn get_skill(name_or_uri: &str) -> Option<&'static BundledSkill> {
    let key = name_or_uri.trim();
    SKILLS
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(key) || s.uri.eq_ignore_ascii_case(key))
}

/// Return a structured JSON representation of a bundled skill, or list all
/// available skills if `name` is empty or `"list"`.
pub fn read_skill_json(name: Option<&str>) -> Result<Value, String> {
    let trimmed = name.map(str::trim).filter(|s| !s.is_empty());
    match trimmed {
        None | Some("list") => Ok(json!({
            "skills": SKILLS.iter().map(|s| json!({
                "name": s.name,
                "uri": s.uri,
                "title": s.title,
                "description": s.description,
            })).collect::<Vec<_>>(),
        })),
        Some(key) => {
            let skill = get_skill(key).ok_or_else(|| {
                let valid: Vec<&str> = SKILLS.iter().map(|s| s.name).collect();
                format!(
                    "unknown skill '{key}'; available skills: {}",
                    valid.join(", ")
                )
            })?;
            Ok(json!({
                "name": skill.name,
                "uri": skill.uri,
                "title": skill.title,
                "description": skill.description,
                "content": skill.content,
            }))
        }
    }
}
