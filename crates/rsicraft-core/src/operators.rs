//! Evolutionary operators and prompt synthesis for RSICraft.
//!
//! Implements the 4-operator action space (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`)
//! with operator-specific sampling temperatures and prompt construction.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchOperator {
    Draft,
    Improve,
    Debug,
    Crossover,
}

impl SearchOperator {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "DRAFT" => Ok(Self::Draft),
            "IMPROVE" => Ok(Self::Improve),
            "DEBUG" => Ok(Self::Debug),
            "CROSSOVER" => Ok(Self::Crossover),
            other => Err(format!(
                "invalid operator '{other}'; expected DRAFT, IMPROVE, DEBUG, or CROSSOVER"
            )),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Improve => "IMPROVE",
            Self::Debug => "DEBUG",
            Self::Crossover => "CROSSOVER",
        }
    }

    /// Target sampling temperature per the Frontis-MA1 / OpenRSI specification.
    #[must_use]
    pub const fn target_temperature(self) -> f64 {
        match self {
            Self::Draft => 0.70,
            Self::Improve => 0.40,
            Self::Debug => 0.20,
            Self::Crossover => 0.50,
        }
    }
}

impl fmt::Display for SearchOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Language a task's candidates are written in. The evaluator script defines
/// the task, so it is the source of truth for the language.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CodeLanguage {
    Python,
    Rust,
}

impl CodeLanguage {
    /// Detect the language from an evaluator script. `None` when the script is blank.
    #[must_use]
    pub fn from_evaluator(evaluator_script: &str) -> Option<Self> {
        if evaluator_script.trim().is_empty() {
            return None;
        }
        let rust = evaluator_script.contains("extern crate candidate")
            || evaluator_script.contains("fn main")
            || evaluator_script.contains("candidate::");
        Some(if rust { Self::Rust } else { Self::Python })
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::Rust => "Rust",
        }
    }

    /// Info string used on markdown code fences.
    #[must_use]
    pub const fn fence(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Rust => "rust",
        }
    }

    const fn contract(self) -> &'static str {
        match self {
            Self::Python => "",
            Self::Rust => " The code is compiled as a library crate named `candidate` using only the Rust standard library, so every item the task asks for must be `pub`.",
        }
    }
}

/// Fence info strings that mark a block as candidate source code.
const CODE_TAGS: &[&str] = &["python", "py", "python3", "rust", "rs"];

pub struct PromptBuilder;

impl PromptBuilder {
    #[must_use]
    pub fn build_system_prompt(op: SearchOperator, lang: CodeLanguage) -> String {
        let role = match op {
            SearchOperator::Draft => {
                "You are an expert autonomous algorithm architect. Your task is to draft a clean, high-performance %LANG% candidate solution based strictly on the problem specification."
            }
            SearchOperator::Improve => {
                "You are a performance optimization expert. Refactor and optimize the provided functional %LANG% solution to increase execution speed and efficiency while maintaining 100% mathematical correctness."
            }
            SearchOperator::Debug => {
                "You are a master debugging engineer. Analyze the provided %LANG% code and its corresponding sandbox failure log or stack trace. Fix all defects and runtime crashes."
            }
            SearchOperator::Crossover => {
                "You are an evolutionary code synthesis engine. Analyze two parent %LANG% solutions targeting the same problem. Recombine the superior structural, algorithmic, and data-structure design elements of both into a unified, superior solution."
            }
        };
        format!(
            "{}{} Output ONLY the complete executable {} code inside one standard ```{} markdown block.",
            role.replace("%LANG%", lang.name()),
            lang.contract(),
            lang.name(),
            lang.fence()
        )
    }

    #[must_use]
    pub fn build_user_prompt(
        op: SearchOperator,
        lang: CodeLanguage,
        task_desc: &str,
        parent_a_code: Option<&str>,
        parent_b_code: Option<&str>,
        execution_logs: Option<&str>,
    ) -> String {
        let fence = lang.fence();
        match op {
            SearchOperator::Draft => {
                format!("### TASK SPECIFICATION\n{task_desc}\n\nGenerate the complete solution.")
            }
            SearchOperator::Improve => {
                format!(
                    "### TASK SPECIFICATION\n{}\n\n### PARENT CODE TO OPTIMIZE\n```{fence}\n{}\n```\n\nProvide the optimized code.",
                    task_desc,
                    parent_a_code.unwrap_or_default()
                )
            }
            SearchOperator::Debug => {
                format!(
                    "### TASK SPECIFICATION\n{}\n\n### FAULTY CODE\n```{fence}\n{}\n```\n\n### EXECUTION FAILURE LOG\n{}\n\nProvide the debugged code.",
                    task_desc,
                    parent_a_code.unwrap_or_default(),
                    execution_logs.unwrap_or("No logs provided.")
                )
            }
            SearchOperator::Crossover => {
                format!(
                    "### TASK SPECIFICATION\n{}\n\n### PARENT A CODE\n```{fence}\n{}\n```\n\n### PARENT B CODE\n```{fence}\n{}\n```\n\nSynthesize the hybrid solution.",
                    task_desc,
                    parent_a_code.unwrap_or_default(),
                    parent_b_code.unwrap_or_default()
                )
            }
        }
    }

    /// Split the text after an opening fence into `(info_string, body)`.
    /// The body ends at the closing fence, or at the end of the text when the
    /// model was cut off before closing it.
    fn split_fence(after_open: &str) -> (&str, &str) {
        let (first_line, rest) = match after_open.find('\n') {
            Some(nl) => (
                after_open.get(..nl).unwrap_or(""),
                after_open.get(nl.saturating_add(1)..).unwrap_or(""),
            ),
            None => ("", after_open),
        };
        let tag = first_line.trim();
        let is_tag = tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '_' | '#' | '.'));
        // A first line that is not a bare info string is already code.
        let (tag, body) = if is_tag {
            (tag, rest)
        } else {
            ("", after_open)
        };
        let body = match body.find("```") {
            Some(end) => body.get(..end).unwrap_or(body),
            None => body,
        };
        (tag, body)
    }

    /// Remove the indentation shared by every non-blank line, then trim.
    /// Models often nest a code fence inside a markdown list, which indents the
    /// whole block and would make Python reject it.
    fn dedent(code: &str) -> String {
        let indent = code
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start_matches([' ', '\t']).len())
            .min()
            .unwrap_or(0);
        if indent == 0 {
            return code.trim().to_string();
        }
        code.lines()
            .map(|l| l.get(indent..).unwrap_or_else(|| l.trim_start()))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    /// Pull the candidate source out of a raw model reply.
    ///
    /// Reasoning preambles (`<|channel>thought...<channel|>`, `<think>...</think>`)
    /// are dropped first. The first fence tagged as Python or Rust wins, then the
    /// first fence of any kind; the fence's info string is never part of the
    /// result. An unterminated fence (reply truncated at the token limit) yields
    /// everything after its opening line. Without any fence the trimmed text is
    /// returned as is.
    #[must_use]
    pub fn extract_code_block(text: &str) -> String {
        let mut body = text;
        if let Some(idx) = body.rfind("<channel|>") {
            if let Some(after) = body.get(idx.saturating_add("<channel|>".len())..) {
                body = after;
            }
        }
        if let Some(idx) = body.rfind("</think>") {
            if let Some(after) = body.get(idx.saturating_add("</think>".len())..) {
                body = after;
            }
        }

        let mut first_any: Option<&str> = None;
        let mut cursor = 0;
        while let Some(rel) = body.get(cursor..).and_then(|s| s.find("```")) {
            let open_end = cursor.saturating_add(rel).saturating_add(3);
            let Some(after_open) = body.get(open_end..) else {
                break;
            };
            let (tag, code) = Self::split_fence(after_open);
            if CODE_TAGS.contains(&tag.to_ascii_lowercase().as_str()) {
                return Self::dedent(code);
            }
            if first_any.is_none() {
                first_any = Some(code);
            }
            // Continue after this block's closing fence, if it has one.
            let Some(close_rel) = after_open.find("```") else {
                break;
            };
            cursor = open_end.saturating_add(close_rel).saturating_add(3);
        }

        Self::dedent(first_any.unwrap_or(body))
    }
}
