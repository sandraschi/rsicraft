//! LLM client for any OpenAI-compatible chat endpoint (Ollama, LM Studio,
//! llama-server, vLLM, hosted APIs), plus a deterministic demo oracle.
//!
//! Failure handling rules:
//! - Every failure is an [`LlmError`] with a class ([`LlmFailure`]), a message
//!   that says what happened, and a remedy that says what to do. Its text form
//!   starts with `[llm:<class>]` so callers holding only a `String` can still
//!   tell the classes apart ([`failure_kind_of`]).
//! - Transient failures are retried a bounded number of times: a server that
//!   is not up yet, or a 5xx that came back quickly. A 5xx after a long
//!   generation is not retried, because repeating it costs minutes and usually
//!   fails the same way.
//! - Setup failures (wrong address, unknown model, missing API key, bad
//!   configuration) are marked fatal: a campaign stops at the first one
//!   instead of burning every remaining step on the same error.
//! - [`LlamaClient::diagnose`] inspects the endpoint (provider, served models,
//!   model availability, risky context settings) and, when nothing answers,
//!   scans the usual local server addresses so the error can name the right one.

use crate::operators::SearchOperator;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;
use std::time::{Duration, Instant};

pub const DEFAULT_LLAMA_URL: &str = "http://127.0.0.1:8081";
pub const DEFAULT_MODEL_ALIAS: &str = "frontis";

/// Completion budget per synthesis call. Reasoning models spend part of it on
/// their thought preamble, so a tight budget truncates the code block.
/// Override with the `RSICRAFT_MAX_TOKENS` environment variable.
pub const DEFAULT_MAX_TOKENS: u32 = 12288;

/// Wall-clock limit for one completion. A 35B reasoning model can need several
/// minutes for one step. Override with `RSICRAFT_LLM_TIMEOUT_SECS`.
pub const DEFAULT_LLM_TIMEOUT_SECS: u64 = 600;

/// Usual local server addresses, scanned when the configured one does not answer.
pub const KNOWN_LOCAL_SERVERS: &[(&str, &str)] = &[
    ("Ollama", "http://127.0.0.1:11434"),
    ("LM Studio", "http://127.0.0.1:1234"),
    ("llama-server", "http://127.0.0.1:8080"),
    ("llama-server", "http://127.0.0.1:8081"),
    ("vLLM or other", "http://127.0.0.1:8000"),
];

/// A 5xx that arrives faster than this is retried; a slower one is not.
const FAST_FAILURE: Duration = Duration::from_secs(30);
/// Pause before retry 1 and retry 2.
const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(2), Duration::from_secs(6)];

/// Class of an LLM failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmFailure {
    /// The client configuration is invalid (malformed URL, wrong path).
    Config,
    /// Nothing answers at the configured address.
    Unreachable,
    /// The server did not finish within the configured time.
    Timeout,
    /// The server does not know the requested model.
    ModelNotFound,
    /// The server wants an API key, or rejected the one given.
    AuthRequired,
    /// Prompt plus completion budget do not fit the model's context window.
    ContextOverflow,
    /// The server failed while handling the request (5xx, dropped connection).
    ServerError,
    /// The server rejected the request for another reason (4xx).
    BadRequest,
    /// The server answered with something that is not a chat completion.
    BadResponse,
    /// The model used the whole completion budget without giving an answer.
    BudgetExhausted,
    /// The model returned an empty answer.
    EmptyAnswer,
    /// The answer contained no program (prose, or a fragment without a function).
    NoCode,
}

impl LlmFailure {
    pub const ALL: [Self; 12] = [
        Self::Config,
        Self::Unreachable,
        Self::Timeout,
        Self::ModelNotFound,
        Self::AuthRequired,
        Self::ContextOverflow,
        Self::ServerError,
        Self::BadRequest,
        Self::BadResponse,
        Self::BudgetExhausted,
        Self::EmptyAnswer,
        Self::NoCode,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Unreachable => "unreachable",
            Self::Timeout => "timeout",
            Self::ModelNotFound => "model_not_found",
            Self::AuthRequired => "auth_required",
            Self::ContextOverflow => "context_overflow",
            Self::ServerError => "server_error",
            Self::BadRequest => "bad_request",
            Self::BadResponse => "bad_response",
            Self::BudgetExhausted => "budget_exhausted",
            Self::EmptyAnswer => "empty_answer",
            Self::NoCode => "no_code",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == raw.trim())
    }

    /// The setup is wrong: every further request will fail the same way, so a
    /// campaign must stop instead of trying the next step.
    #[must_use]
    pub const fn is_fatal(self) -> bool {
        matches!(
            self,
            Self::Config | Self::Unreachable | Self::ModelNotFound | Self::AuthRequired
        )
    }
}

/// A classified LLM failure with a remedy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmError {
    pub kind: LlmFailure,
    pub message: String,
    pub remedy: String,
    pub status: Option<u16>,
    pub attempts: u32,
}

impl LlmError {
    #[must_use]
    pub fn new(kind: LlmFailure, message: impl Into<String>, remedy: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            remedy: remedy.into(),
            status: None,
            attempts: 1,
        }
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "fatal": self.kind.is_fatal(),
            "message": self.message,
            "remedy": self.remedy,
            "status": self.status,
            "attempts": self.attempts,
        })
    }
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[llm:{}] {}", self.kind.as_str(), self.message)?;
        if self.attempts > 1 {
            write!(f, " (after {} attempts)", self.attempts)?;
        }
        write!(f, " | fix: {}", self.remedy)
    }
}

impl std::error::Error for LlmError {}

impl From<LlmError> for String {
    fn from(e: LlmError) -> Self {
        e.to_string()
    }
}

/// Class of an error string produced by this module, if it is one.
#[must_use]
pub fn failure_kind_of(err: &str) -> Option<LlmFailure> {
    let start = err.find("[llm:")? + "[llm:".len();
    let rest = err.get(start..)?;
    let end = rest.find(']')?;
    LlmFailure::parse(rest.get(..end)?)
}

/// Normalize a user-supplied base URL to the server root: trims whitespace and
/// trailing slashes, and drops a trailing `/v1` or `/v1/chat/completions`
/// (people often paste the full endpoint).
pub fn normalize_base_url(raw: &str) -> Result<String, LlmError> {
    let mut base = raw.trim().trim_end_matches('/').to_string();
    for suffix in ["/v1/chat/completions", "/chat/completions", "/v1"] {
        if base.to_ascii_lowercase().ends_with(suffix) {
            base.truncate(base.len() - suffix.len());
            base = base.trim_end_matches('/').to_string();
        }
    }
    let lower = base.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) || base.len() > 253 {
        return Err(LlmError::new(
            LlmFailure::Config,
            format!("'{raw}' is not a usable server address"),
            "pass the server root, for example --llama-url http://127.0.0.1:11434 (Ollama) or http://127.0.0.1:1234 (LM Studio), or set RSICRAFT_LLM_URL",
        ));
    }
    Ok(base)
}

/// Flatten and clip a response body for an error message.
pub(crate) fn format_status_failure(endpoint: &str, code: u16, body: &str) -> String {
    let mut flat = String::new();
    for ch in body.chars() {
        if flat.chars().count() >= 240 {
            break;
        }
        if ch != '\r' {
            flat.push(if ch == '\n' { ' ' } else { ch });
        }
    }
    let flat = flat.trim();
    if flat.is_empty() {
        format!("llama-server request to {endpoint} failed: status code {code}")
    } else {
        format!("llama-server request to {endpoint} failed: status code {code}: {flat}")
    }
}

/// Classify a non-2xx response from its status code and body.
#[must_use]
pub fn classify_status(code: u16, body: &str) -> LlmFailure {
    let b = body.to_ascii_lowercase();
    let model_missing = (b.contains("model")
        && (b.contains("not found")
            || b.contains("does not exist")
            || b.contains("not loaded")
            || b.contains("unknown model")))
        || b.contains("no models loaded")
        || b.contains("try pulling");
    let context = b.contains("n_ctx")
        || b.contains("maximum context")
        || (b.contains("context")
            && (b.contains("exceed")
                || b.contains("too long")
                || b.contains("overflow")
                || b.contains("too large")));
    match code {
        401 | 403 => LlmFailure::AuthRequired,
        _ if model_missing => LlmFailure::ModelNotFound,
        _ if context => LlmFailure::ContextOverflow,
        404 | 405 => LlmFailure::Config,
        400..=499 => LlmFailure::BadRequest,
        _ => LlmFailure::ServerError,
    }
}

fn configured_timeout_secs() -> u64 {
    std::env::var("RSICRAFT_LLM_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| (5..=7200).contains(v))
        .unwrap_or(DEFAULT_LLM_TIMEOUT_SECS)
}

/// Completion budget actually sent to the server.
#[must_use]
pub fn configured_max_tokens() -> u32 {
    std::env::var("RSICRAFT_MAX_TOKENS")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|v| (64..=65_536).contains(v))
        .unwrap_or(DEFAULT_MAX_TOKENS)
}

fn api_key() -> Option<String> {
    std::env::var("RSICRAFT_LLM_API_KEY")
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
}

/// True when `requested` names the served model `id` (case-insensitive;
/// Ollama ids carry a `:tag`, LM Studio ids may carry a `publisher/` prefix).
#[must_use]
pub fn model_matches(requested: &str, id: &str) -> bool {
    let r = requested.trim().to_ascii_lowercase();
    let i = id.trim().to_ascii_lowercase();
    if r.is_empty() {
        return false;
    }
    r == i
        || (!r.contains(':') && i.split(':').next() == Some(r.as_str()))
        || i.ends_with(&format!("/{r}"))
        || (!r.contains(':') && i.ends_with(&format!("/{r}:latest")))
}

/// Remedy text for a failure class.
fn remedy_for(
    kind: LlmFailure,
    base: &str,
    model: &str,
    timeout_secs: u64,
    elapsed: Duration,
) -> String {
    let max_tokens = configured_max_tokens();
    match kind {
        LlmFailure::Config => format!(
            "check --llama-url: it must be the server root that serves /v1/chat/completions (now '{base}'). Run `rsicraft probe` to check it"
        ),
        LlmFailure::Unreachable => format!(
            "nothing answers at {base}. Start the server, or point rsicraft at the right one with --llama-url or RSICRAFT_LLM_URL. Usual addresses: Ollama http://127.0.0.1:11434, LM Studio http://127.0.0.1:1234 (start the server in its Developer tab), llama-server http://127.0.0.1:8080. `rsicraft probe` scans them"
        ),
        LlmFailure::Timeout => format!(
            "the model did not finish within {timeout_secs}s. Raise RSICRAFT_LLM_TIMEOUT_SECS, lower RSICRAFT_MAX_TOKENS (now {max_tokens}), set RSICRAFT_REASONING_EFFORT=low, or use a smaller model"
        ),
        LlmFailure::ModelNotFound => format!(
            "the server does not serve model '{model}'. Ollama: `ollama list`, then `ollama pull <name>`. LM Studio: download and load the model, or turn on just-in-time loading. Then pass the exact name with --model or RSICRAFT_MODEL. `rsicraft probe` lists what the server offers"
        ),
        LlmFailure::AuthRequired => {
            "the endpoint needs an API key. Set RSICRAFT_LLM_API_KEY (sent as a Bearer token), or check that the key is valid".to_string()
        }
        LlmFailure::ContextOverflow => format!(
            "prompt plus completion budget ({max_tokens} tokens) exceed the model's context window. Give the model a larger context (Ollama: an alias with PARAMETER num_ctx; LM Studio: Context Length when loading; llama-server: --ctx-size) or lower RSICRAFT_MAX_TOKENS"
        ),
        LlmFailure::ServerError => {
            if elapsed >= FAST_FAILURE {
                format!(
                    "the server failed after {}s of generation. This is usually the model runner crashing at the completion budget or running out of memory: check the server log, lower RSICRAFT_MAX_TOKENS (now {max_tokens}), or give the model a smaller context",
                    elapsed.as_secs()
                )
            } else {
                "the server failed straight away. Check its log; if it was loading or swapping models, run the step again".to_string()
            }
        }
        LlmFailure::BadRequest => format!(
            "the server rejected the request. Check that --llama-url ({base}) is an OpenAI-compatible server and that '{model}' is a chat model"
        ),
        LlmFailure::BadResponse => format!(
            "the answer from {base} is not an OpenAI chat completion. Check that --llama-url points to the server root of an OpenAI-compatible server"
        ),
        LlmFailure::BudgetExhausted => format!(
            "the model used all {max_tokens} completion tokens without answering. Raise RSICRAFT_MAX_TOKENS (and the model context with it), set RSICRAFT_REASONING_EFFORT=low, or use a model that thinks less"
        ),
        LlmFailure::EmptyAnswer => {
            "the model returned an empty answer. Run the step again; if it keeps happening, try another model".to_string()
        }
        LlmFailure::NoCode => {
            "the answer held no complete program. The model may be too weak for the task, or it ran out of budget mid-answer: try a stronger model or a larger RSICRAFT_MAX_TOKENS".to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareProfile {
    pub host_identifier: String,
    pub cpu: String,
    pub ram_gb: u32,
    pub gpu: String,
    pub vram_total_gb: f64,
    pub model_architecture: String,
    pub active_parameters_b: f64,
    pub quantization: String,
    pub kv_cache_type: String,
    pub flash_attention: bool,
    pub context_window_tokens: u32,
    pub vram_allocated_min_gb: f64,
    pub vram_allocated_max_gb: f64,
    pub vram_headroom_gb: f64,
    pub llama_server_url: String,
    pub llama_server_command: String,
}

/// Reference hardware profile (the author's machine). Static text, not detected.
#[must_use]
pub fn goliath_4090_profile(llama_url: &str) -> HardwareProfile {
    HardwareProfile {
        host_identifier: "Goliath".into(),
        cpu: "AMD Ryzen 9 7900X / Threadripper Architecture (24 threads)".into(),
        ram_gb: 64,
        gpu: "NVIDIA GeForce RTX 4090".into(),
        vram_total_gb: 24.0,
        model_architecture: "Frontis-MA1 30B/35B MoE".into(),
        active_parameters_b: 3.0,
        quantization: "Q4_K_M / Q5_K_M".into(),
        kv_cache_type: "q4_0".into(),
        flash_attention: true,
        context_window_tokens: 16384,
        vram_allocated_min_gb: 18.0,
        vram_allocated_max_gb: 19.5,
        vram_headroom_gb: 4.5,
        llama_server_url: llama_url.to_string(),
        llama_server_command: "llama-server --model ./models/frontis-ma1-30b-q4_k_m.gguf --alias frontis --n-gpu-layers 99 --ctx-size 16384 --port 8081 --flash-attn --cache-type-k q4_0".into(),
    }
}

/// The last closed Python or Rust code fence in `text` whose body defines a
/// function or imports something. Used only to salvage an answer from a
/// reasoning field when the model never produced its final answer.
fn last_closed_code_fence(text: &str) -> Option<String> {
    const TAGS: [&str; 4] = ["python", "py", "rust", "rs"];
    let mut found: Option<String> = None;
    let mut rest = text;
    while let Some(open) = rest.find("```") {
        let after_open = rest.get(open + 3..)?;
        let (tag_line, body_and_more) = match after_open.find('\n') {
            Some(nl) => (after_open.get(..nl)?, after_open.get(nl + 1..)?),
            None => break,
        };
        let Some(close) = body_and_more.find("```") else {
            break;
        };
        let body = body_and_more.get(..close)?;
        let tag = tag_line.trim().to_ascii_lowercase();
        let defines = body.lines().any(|l| {
            let t = l.trim_start();
            t.starts_with("def ")
                || t.starts_with("async def ")
                || t.starts_with("fn ")
                || t.starts_with("pub fn ")
        });
        if TAGS.contains(&tag.as_str()) && defines {
            found = Some(format!("```{tag}\n{}\n```", body.trim_end()));
        }
        rest = body_and_more.get(close + 3..)?;
    }
    found
}

fn get_json(agent: &ureq::Agent, url: &str) -> Option<Value> {
    let mut req = agent.get(url);
    if let Some(k) = api_key() {
        req = req.set("Authorization", &format!("Bearer {k}"));
    }
    req.call().ok()?.into_json::<Value>().ok()
}

fn quick_agent(connect: Duration, total: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(connect)
        .timeout(total)
        .build()
}

/// Which kind of server answers at a base URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Ollama,
    LmStudio,
    LlamaServer,
    OpenAiCompatible,
}

impl ProviderKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::LmStudio => "LM Studio",
            Self::LlamaServer => "llama-server",
            Self::OpenAiCompatible => "OpenAI-compatible server",
        }
    }
}

fn detect_provider(base: &str) -> ProviderKind {
    let agent = quick_agent(Duration::from_millis(800), Duration::from_secs(2));
    if get_json(&agent, &format!("{base}/api/version")).is_some_and(|v| v.get("version").is_some())
    {
        return ProviderKind::Ollama;
    }
    if get_json(&agent, &format!("{base}/api/v0/models"))
        .is_some_and(|v| v.get("data").is_some_and(Value::is_array))
    {
        return ProviderKind::LmStudio;
    }
    if get_json(&agent, &format!("{base}/props")).is_some_and(|v| {
        v.get("default_generation_settings").is_some() || v.get("model_path").is_some()
    }) {
        return ProviderKind::LlamaServer;
    }
    ProviderKind::OpenAiCompatible
}

/// Context window Ollama will use for `model`, from `num_ctx` in the model's
/// parameters, and the model's native maximum (`*.context_length`).
fn ollama_context(base: &str, model: &str) -> (Option<u64>, Option<u64>) {
    let agent = quick_agent(Duration::from_millis(800), Duration::from_secs(5));
    let Ok(resp) = agent
        .post(&format!("{base}/api/show"))
        .send_json(json!({ "model": model }))
    else {
        return (None, None);
    };
    let Ok(v) = resp.into_json::<Value>() else {
        return (None, None);
    };
    let num_ctx = v.get("parameters").and_then(Value::as_str).and_then(|p| {
        p.lines().find_map(|l| {
            let mut it = l.split_whitespace();
            (it.next() == Some("num_ctx"))
                .then(|| it.next().and_then(|n| n.parse::<u64>().ok()))
                .flatten()
        })
    });
    let native = v
        .get("model_info")
        .and_then(Value::as_object)
        .and_then(|m| {
            m.iter()
                .find(|(k, _)| k.ends_with(".context_length"))
                .and_then(|(_, n)| n.as_u64())
        });
    (num_ctx, native)
}

#[derive(Debug, Clone)]
pub struct LlamaClient {
    pub base_url: String,
    pub model_alias: String,
    pub timeout_secs: u64,
}

impl Default for LlamaClient {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_LLAMA_URL.to_string(),
            model_alias: DEFAULT_MODEL_ALIAS.to_string(),
            timeout_secs: configured_timeout_secs(),
        }
    }
}

impl LlamaClient {
    #[must_use]
    pub fn new(base_url: impl Into<String>, model_alias: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model_alias: model_alias.into(),
            timeout_secs: configured_timeout_secs(),
        }
    }

    fn error(&self, kind: LlmFailure, message: impl Into<String>, elapsed: Duration) -> LlmError {
        let base = normalize_base_url(&self.base_url).unwrap_or_else(|_| self.base_url.clone());
        LlmError::new(
            kind,
            message,
            remedy_for(kind, &base, &self.model_alias, self.timeout_secs, elapsed),
        )
    }

    /// Send one chat completion request and return the model's answer text.
    /// Transient failures are retried; see the module documentation.
    pub fn complete(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f64,
    ) -> Result<String, LlmError> {
        let base = normalize_base_url(&self.base_url)?;
        let endpoint = format!("{base}/v1/chat/completions");
        let mut payload = json!({
            "model": self.model_alias,
            "temperature": temperature,
            "max_tokens": configured_max_tokens(),
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ]
        });
        // Optional hint for reasoning models (Ollama and other OpenAI-compatible
        // servers accept it; llama-server ignores unknown fields).
        if let Ok(effort) = std::env::var("RSICRAFT_REASONING_EFFORT") {
            if matches!(effort.trim(), "low" | "medium" | "high" | "none") {
                payload["reasoning_effort"] = json!(effort.trim());
            }
        }

        let mut attempts = 0u32;
        loop {
            attempts += 1;
            let started = Instant::now();
            let result = self
                .post_once(&endpoint, &payload, started)
                .and_then(|body| self.read_answer(&body, started));
            match result {
                Ok(text) => return Ok(text),
                Err(mut e) => {
                    let retry_allowed = (attempts as usize) <= RETRY_DELAYS.len()
                        && match e.kind {
                            LlmFailure::Unreachable => attempts == 1,
                            LlmFailure::ServerError => started.elapsed() < FAST_FAILURE,
                            _ => false,
                        };
                    if retry_allowed {
                        if let Some(delay) = RETRY_DELAYS.get(attempts as usize - 1) {
                            std::thread::sleep(*delay);
                        }
                        continue;
                    }
                    e.attempts = attempts;
                    return Err(e);
                }
            }
        }
    }

    fn post_once(
        &self,
        endpoint: &str,
        payload: &Value,
        started: Instant,
    ) -> Result<Value, LlmError> {
        let agent = quick_agent(
            Duration::from_secs(5),
            Duration::from_secs(self.timeout_secs),
        );
        let mut req = agent.post(endpoint).set("Content-Type", "application/json");
        if let Some(k) = api_key() {
            req = req.set("Authorization", &format!("Bearer {k}"));
        }
        match req.send_json(payload.clone()) {
            Ok(resp) => resp.into_json::<Value>().map_err(|e| {
                self.error(
                    LlmFailure::BadResponse,
                    format!("the answer from {endpoint} is not JSON: {e}"),
                    started.elapsed(),
                )
            }),
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                let kind = classify_status(code, &body);
                let mut e = self.error(
                    kind,
                    format!(
                        "{} (after {}s)",
                        format_status_failure(endpoint, code, &body),
                        started.elapsed().as_secs()
                    ),
                    started.elapsed(),
                );
                e.status = Some(code);
                Err(e)
            }
            Err(ureq::Error::Transport(t)) => {
                let text = t.to_string();
                let lower = text.to_ascii_lowercase();
                let kind = match t.kind() {
                    ureq::ErrorKind::InvalidUrl | ureq::ErrorKind::UnknownScheme => {
                        LlmFailure::Config
                    }
                    ureq::ErrorKind::Dns | ureq::ErrorKind::ConnectionFailed => {
                        LlmFailure::Unreachable
                    }
                    _ if lower.contains("timed out") || lower.contains("timeout") => {
                        LlmFailure::Timeout
                    }
                    _ if lower.contains("reset")
                        || lower.contains("aborted")
                        || lower.contains("closed")
                        || lower.contains("broken pipe")
                        || lower.contains("eof") =>
                    {
                        LlmFailure::ServerError
                    }
                    _ => LlmFailure::Unreachable,
                };
                Err(self.error(
                    kind,
                    format!(
                        "request to {endpoint} failed after {}s: {text}",
                        started.elapsed().as_secs()
                    ),
                    started.elapsed(),
                ))
            }
        }
    }

    fn read_answer(&self, body: &Value, started: Instant) -> Result<String, LlmError> {
        if let Some(err) = body.get("error") {
            let text = err
                .get("message")
                .and_then(Value::as_str)
                .map_or_else(|| err.to_string(), str::to_string);
            let kind = match classify_status(400, &text) {
                LlmFailure::BadRequest => LlmFailure::BadResponse,
                other => other,
            };
            return Err(self.error(
                kind,
                format!("the server returned an error: {text}"),
                started.elapsed(),
            ));
        }
        let Some(choice) = body
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
        else {
            return Err(self.error(
                LlmFailure::BadResponse,
                "the answer has no choices[0]",
                started.elapsed(),
            ));
        };
        let message = choice.get("message");
        let field = |key: &str| {
            message
                .and_then(|m| m.get(key))
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
        };
        if let Some(content) = field("content") {
            return Ok(content.to_string());
        }
        // A reasoning model that spent the budget thinking may still have
        // written a complete program inside its reasoning. Only a closed code
        // block that defines a function counts; quoted fragments do not.
        let reasoning = field("reasoning").or_else(|| field("reasoning_content"));
        if let Some(code) = reasoning.and_then(last_closed_code_fence) {
            return Ok(code);
        }
        let finish = choice
            .get("finish_reason")
            .and_then(Value::as_str)
            .unwrap_or("");
        let reasoned = reasoning.map_or(0, str::len);
        if finish == "length" {
            return Err(self.error(
                LlmFailure::BudgetExhausted,
                format!(
                    "no answer: the model stopped at the completion budget after {}s ({reasoned} characters of reasoning, no complete code block)",
                    started.elapsed().as_secs()
                ),
                started.elapsed(),
            ));
        }
        Err(self.error(
            LlmFailure::EmptyAnswer,
            format!("empty answer (finish_reason '{finish}', {reasoned} characters of reasoning)"),
            started.elapsed(),
        ))
    }

    /// Inspect the endpoint and the configured model without generating text.
    /// Never fails: problems are reported inside the returned JSON
    /// (`ok`, `failure`, `warnings`).
    #[must_use]
    pub fn diagnose(&self) -> Value {
        let (report, _) = self.inspect();
        report
    }

    /// Like [`diagnose`](Self::diagnose), but turns a problem that would make
    /// every request fail into an error. Warnings do not fail the preflight.
    pub fn preflight(&self) -> Result<Value, LlmError> {
        match self.inspect() {
            (_, Some(e)) => Err(e),
            (report, None) => Ok(report),
        }
    }

    fn inspect(&self) -> (Value, Option<LlmError>) {
        let started = Instant::now();
        let model = self.model_alias.clone();
        let mut warnings: Vec<String> = Vec::new();
        let report = |ok: bool,
                      base: &str,
                      provider: Option<ProviderKind>,
                      models: &[String],
                      available: Option<bool>,
                      warnings: &[String],
                      failure: Option<&LlmError>,
                      found: &[Value]| {
            json!({
                "ok": ok,
                "base_url": base,
                "provider": provider.map(ProviderKind::label),
                "configured_model": model,
                "model_available": available,
                "available_models": models.iter().take(50).collect::<Vec<_>>(),
                "max_tokens": configured_max_tokens(),
                "timeout_secs": self.timeout_secs,
                "api_key_set": api_key().is_some(),
                "latency_ms": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                "warnings": warnings,
                "failure": failure.map(LlmError::to_json),
                "other_servers_found": found,
            })
        };

        let base = match normalize_base_url(&self.base_url) {
            Ok(b) => b,
            Err(e) => {
                return (
                    report(false, &self.base_url, None, &[], None, &[], Some(&e), &[]),
                    Some(e),
                )
            }
        };
        if base != self.base_url.trim().trim_end_matches('/') {
            warnings.push(format!(
                "the configured address '{}' was shortened to the server root '{base}'",
                self.base_url
            ));
        }

        let agent = quick_agent(Duration::from_secs(2), Duration::from_secs(6));
        let mut req = agent.get(&format!("{base}/v1/models"));
        if let Some(k) = api_key() {
            req = req.set("Authorization", &format!("Bearer {k}"));
        }
        let models: Vec<String> = match req.call() {
            Ok(resp) => resp
                .into_json::<Value>()
                .ok()
                .and_then(|v| v.get("data").and_then(Value::as_array).cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string))
                .collect(),
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                let kind = match code {
                    401 | 403 => LlmFailure::AuthRequired,
                    404 | 405 => LlmFailure::Config,
                    _ => LlmFailure::ServerError,
                };
                let mut e = self.error(
                    kind,
                    format_status_failure(&format!("{base}/v1/models"), code, &body),
                    started.elapsed(),
                );
                e.status = Some(code);
                if kind.is_fatal() {
                    return (
                        report(false, &base, None, &[], None, &warnings, Some(&e), &[]),
                        Some(e),
                    );
                }
                warnings.push(format!("listing models failed: {}", e.message));
                Vec::new()
            }
            Err(ureq::Error::Transport(t)) => {
                let found: Vec<Value> = KNOWN_LOCAL_SERVERS
                    .iter()
                    .filter(|(_, url)| *url != base)
                    .filter(|(_, url)| {
                        quick_agent(Duration::from_millis(400), Duration::from_secs(2))
                            .get(&format!("{url}/v1/models"))
                            .call()
                            .is_ok()
                    })
                    .map(|(name, url)| json!({ "server": name, "url": url }))
                    .collect();
                let mut e = self.error(
                    LlmFailure::Unreachable,
                    format!("nothing answers at {base}: {t}"),
                    started.elapsed(),
                );
                if !found.is_empty() {
                    let list: Vec<String> = found
                        .iter()
                        .map(|f| {
                            format!(
                                "{} at {}",
                                f["server"].as_str().unwrap_or("?"),
                                f["url"].as_str().unwrap_or("?")
                            )
                        })
                        .collect();
                    e.remedy = format!(
                        "found a server at another address: {}. Use it with --llama-url <address> or RSICRAFT_LLM_URL",
                        list.join(", ")
                    );
                }
                return (
                    report(false, &base, None, &[], None, &warnings, Some(&e), &found),
                    Some(e),
                );
            }
        };

        let provider = detect_provider(&base);
        let available = models.iter().any(|id| model_matches(&model, id));
        let mut failure: Option<LlmError> = None;
        if !available {
            match provider {
                ProviderKind::LlamaServer => warnings.push(format!(
                    "llama-server serves one model and ignores the model name; '{model}' is not in its list ({}), requests will go to the loaded model",
                    models.join(", ")
                )),
                _ => {
                    let listed = if models.is_empty() {
                        "the server lists no models".to_string()
                    } else {
                        let mut shown: Vec<&str> = models.iter().take(12).map(String::as_str).collect();
                        if models.len() > 12 {
                            shown.push("...");
                        }
                        format!("served models: {}", shown.join(", "))
                    };
                    failure = Some(self.error(
                        LlmFailure::ModelNotFound,
                        format!("{} at {base} does not serve '{model}'; {listed}", provider.label()),
                        started.elapsed(),
                    ));
                }
            }
        }

        let max_tokens = u64::from(configured_max_tokens());
        if failure.is_none() && provider == ProviderKind::Ollama {
            let (num_ctx, native) = ollama_context(&base, &model);
            match (num_ctx, native) {
                (Some(n), _) if n < max_tokens + 2048 => warnings.push(format!(
                    "Ollama context for '{model}' is {n} tokens, smaller than the completion budget {max_tokens} plus a prompt: long answers will be cut off or fail. Raise num_ctx or lower RSICRAFT_MAX_TOKENS"
                )),
                (None, Some(n)) if n > 32_768 => warnings.push(format!(
                    "'{model}' sets no num_ctx; Ollama may allocate its full {n}-token context, which can overflow GPU memory and run on the CPU. Create an alias with PARAMETER num_ctx 16384 (see help tab 'providers')"
                )),
                _ => {}
            }
        }
        if failure.is_none() && provider == ProviderKind::LmStudio {
            let agent = quick_agent(Duration::from_millis(800), Duration::from_secs(3));
            if let Some(entry) = get_json(&agent, &format!("{base}/api/v0/models"))
                .and_then(|v| v.get("data").and_then(Value::as_array).cloned())
                .and_then(|list| {
                    list.into_iter().find(|m| {
                        m.get("id")
                            .and_then(Value::as_str)
                            .is_some_and(|id| model_matches(&model, id))
                    })
                })
            {
                if entry.get("state").and_then(Value::as_str) != Some("loaded") {
                    warnings.push(format!(
                        "'{model}' is not loaded in LM Studio. It loads on the first request if just-in-time loading is on (the default); otherwise load it in the app or with `lms load`"
                    ));
                }
                if let Some(ctx) = entry.get("loaded_context_length").and_then(Value::as_u64) {
                    if ctx < max_tokens + 2048 {
                        warnings.push(format!(
                            "LM Studio loaded '{model}' with a {ctx}-token context, smaller than the completion budget {max_tokens} plus a prompt. Reload it with a larger Context Length or lower RSICRAFT_MAX_TOKENS"
                        ));
                    }
                }
            }
        }

        let ok = failure.is_none();
        let value = report(
            ok,
            &base,
            Some(provider),
            &models,
            Some(available),
            &warnings,
            failure.as_ref(),
            &[],
        );
        (value, failure)
    }

    /// Backwards-compatible name for [`diagnose`](Self::diagnose).
    pub fn probe(&self) -> Result<Value, String> {
        Ok(self.diagnose())
    }

    /// Deterministic offline synthesis oracle (`provider = "demo"`).
    /// Produces realistic evolutionary progressions across `DRAFT`, `DEBUG`,
    /// `IMPROVE`, and `CROSSOVER` without requiring live GPU weights.
    #[must_use]
    pub fn demo_synthesize(op: SearchOperator, task_id: &str, step_index: u32) -> String {
        match op {
            SearchOperator::Draft => format!(
                "```python\n# RSICraft Demo DRAFT candidate for task '{task_id}' (step {step_index})\ndef solve(arr):\n    if not arr:\n        return 0.0\n    best = arr[0]\n    for item in arr:\n        if item > best:\n            best = item\n    return float(best)\n```"
            ),
            SearchOperator::Debug => format!(
                "```python\n# RSICraft Demo DEBUG patched candidate for task '{task_id}' (step {step_index})\ndef solve(arr):\n    if arr is None or len(arr) == 0:\n        return 0.0\n    cleaned = [float(x) for x in arr if x is not None]\n    return max(cleaned) if cleaned else 0.0\n```"
            ),
            SearchOperator::Improve => format!(
                "```python\n# RSICraft Demo IMPROVE optimized candidate for task '{task_id}' (step {step_index})\ndef solve(arr):\n    # Vectorized C-loop builtin max with fast empty guard\n    return float(max(arr)) if arr else 0.0\n```"
            ),
            SearchOperator::Crossover => format!(
                "```python\n# RSICraft Demo CROSSOVER hybrid candidate for task '{task_id}' (step {step_index})\ndef solve(arr):\n    # Hybrid combining defensive sanitization from Parent A with builtin speed from Parent B\n    if not arr:\n        return 0.0\n    return float(max(x for x in arr if x is not None))\n```"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_classification_covers_the_common_servers() {
        assert_eq!(classify_status(401, ""), LlmFailure::AuthRequired);
        // Ollama
        assert_eq!(
            classify_status(
                404,
                r#"{"error":"model \"x\" not found, try pulling it first"}"#
            ),
            LlmFailure::ModelNotFound
        );
        // LM Studio
        assert_eq!(
            classify_status(400, r#"{"error":"No models loaded. Please load a model"}"#),
            LlmFailure::ModelNotFound
        );
        // llama-server
        assert_eq!(
            classify_status(
                400,
                "the request exceeds the available context size (n_ctx)"
            ),
            LlmFailure::ContextOverflow
        );
        assert_eq!(classify_status(404, "Not Found"), LlmFailure::Config);
        assert_eq!(classify_status(422, "bad field"), LlmFailure::BadRequest);
        assert_eq!(
            classify_status(500, "runner stopped"),
            LlmFailure::ServerError
        );
    }

    #[test]
    fn error_text_round_trips_its_class_and_names_a_fix() {
        let e = LlmError::new(LlmFailure::ModelNotFound, "no such model", "pull it");
        let s: String = e.into();
        assert!(s.starts_with("[llm:model_not_found] no such model"));
        assert!(s.contains("| fix: pull it"));
        assert_eq!(failure_kind_of(&s), Some(LlmFailure::ModelNotFound));
        assert_eq!(failure_kind_of("plain error"), None);
        for k in LlmFailure::ALL {
            assert_eq!(LlmFailure::parse(k.as_str()), Some(k));
        }
        assert!(LlmFailure::Unreachable.is_fatal());
        assert!(!LlmFailure::BudgetExhausted.is_fatal());
    }

    #[test]
    fn base_urls_are_normalized_and_validated() {
        assert_eq!(
            normalize_base_url(" http://127.0.0.1:11434/v1/ ")
                .ok()
                .as_deref(),
            Some("http://127.0.0.1:11434")
        );
        assert_eq!(
            normalize_base_url("http://h:1234/v1/chat/completions")
                .ok()
                .as_deref(),
            Some("http://h:1234")
        );
        assert_eq!(
            normalize_base_url("127.0.0.1:11434").map_err(|e| e.kind),
            Err(LlmFailure::Config)
        );
    }

    #[test]
    fn model_names_match_ollama_tags_and_lm_studio_prefixes() {
        assert!(model_matches("rsi-qwen9b", "rsi-qwen9b:latest"));
        assert!(model_matches("rsi-qwen9b:latest", "rsi-qwen9b:latest"));
        assert!(!model_matches("rsi-qwen9b:q8", "rsi-qwen9b:latest"));
        assert!(model_matches(
            "qwen2.5-7b-instruct",
            "lmstudio-community/qwen2.5-7b-instruct"
        ));
        assert!(model_matches("Gemma4:12B", "gemma4:12b"));
        assert!(!model_matches("", "gemma4:12b"));
    }

    #[test]
    fn reasoning_salvage_ignores_quoted_fragments() {
        // The failure seen in the pack-v2 run: the model quotes the prompt.
        let quoted = "I must answer inside one ```python markdown block. So the plan is ...";
        assert_eq!(last_closed_code_fence(quoted), None);
        let unclosed = "Draft:\n```python\ndef solve(x):\n    return x";
        assert_eq!(last_closed_code_fence(unclosed), None);
        let fragment = "```python\nfor i in range(3):\n    pass\n```";
        assert_eq!(last_closed_code_fence(fragment), None);
        let two = "```python\ndef solve(x):\n    return 1\n```\nbetter:\n```python\ndef solve(x):\n    return 2\n```";
        assert_eq!(
            last_closed_code_fence(two).as_deref(),
            Some("```python\ndef solve(x):\n    return 2\n```")
        );
    }

    #[test]
    fn a_dead_endpoint_is_classified_fatal_with_a_fix() {
        // Port 9 (discard) is closed on any normal machine: connection refused.
        let client = LlamaClient::new("http://127.0.0.1:9", "anything");
        let err = client.complete("s", "u", 0.1).err();
        let err = err.map(|e| (e.kind, e.attempts, e.remedy.is_empty()));
        assert_eq!(err, Some((LlmFailure::Unreachable, 2, false)));
        let report = client.diagnose();
        assert_eq!(report["ok"], false);
        assert_eq!(report["failure"]["kind"], "unreachable");
        assert_eq!(report["failure"]["fatal"], true);
        assert!(client.preflight().is_err());
    }
}
