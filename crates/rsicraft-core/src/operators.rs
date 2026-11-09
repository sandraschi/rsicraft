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

pub struct PromptBuilder;

impl PromptBuilder {
    #[must_use]
    pub const fn build_system_prompt(op: SearchOperator) -> &'static str {
        match op {
            SearchOperator::Draft => {
                "You are an expert autonomous algorithm architect. Your task is to draft a clean, high-performance Python candidate solution based strictly on the problem specification. Output ONLY executable Python code inside standard ```python markdown blocks."
            }
            SearchOperator::Improve => {
                "You are a performance optimization expert. Refactor and optimize the provided functional solution to increase execution speed and efficiency while maintaining 100% mathematical correctness. Output ONLY executable Python code inside standard ```python markdown blocks."
            }
            SearchOperator::Debug => {
                "You are a master debugging engineer. Analyze the provided Python code and its corresponding sandbox failure log or stack trace. Fix all defects and runtime crashes. Output ONLY executable Python code inside standard ```python markdown blocks."
            }
            SearchOperator::Crossover => {
                "You are an evolutionary code synthesis engine. Analyze two parent Python solutions targeting the same problem. Recombine the superior structural, algorithmic, and data-structure design elements of both into a unified, superior solution. Output ONLY executable Python code inside standard ```python markdown blocks."
            }
        }
    }

    #[must_use]
    pub fn build_user_prompt(
        op: SearchOperator,
        task_desc: &str,
        parent_a_code: Option<&str>,
        parent_b_code: Option<&str>,
        execution_logs: Option<&str>,
    ) -> String {
        match op {
            SearchOperator::Draft => {
                format!("### TASK SPECIFICATION\n{task_desc}\n\nGenerate the complete solution.")
            }
            SearchOperator::Improve => {
                format!(
                    "### TASK SPECIFICATION\n{}\n\n### PARENT CODE TO OPTIMIZE\n```python\n{}\n```\n\nProvide the optimized code.",
                    task_desc,
                    parent_a_code.unwrap_or_default()
                )
            }
            SearchOperator::Debug => {
                format!(
                    "### TASK SPECIFICATION\n{}\n\n### FAULTY CODE\n```python\n{}\n```\n\n### EXECUTION FAILURE LOG\n{}\n\nProvide the debugged code.",
                    task_desc,
                    parent_a_code.unwrap_or_default(),
                    execution_logs.unwrap_or("No logs provided.")
                )
            }
            SearchOperator::Crossover => {
                format!(
                    "### TASK SPECIFICATION\n{}\n\n### PARENT A CODE\n```python\n{}\n```\n\n### PARENT B CODE\n```python\n{}\n```\n\nSynthesize the hybrid solution.",
                    task_desc,
                    parent_a_code.unwrap_or_default(),
                    parent_b_code.unwrap_or_default()
                )
            }
        }
    }

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

        if let Some(start) = body.find("```python") {
            let after_fence = start.saturating_add(9);
            if let Some(code_part) = body.get(after_fence..) {
                if let Some(end) = code_part.find("```") {
                    if let Some(extracted) = code_part.get(..end) {
                        return extracted.trim().to_string();
                    }
                }
            }
        } else if let Some(start) = body.find("```") {
            let after_fence = start.saturating_add(3);
            if let Some(code_part) = body.get(after_fence..) {
                if let Some(end) = code_part.find("```") {
                    if let Some(extracted) = code_part.get(..end) {
                        return extracted.trim().to_string();
                    }
                }
            }
        }
        body.trim().to_string()
    }
}
