use std::str::FromStr;

use clap::ValueEnum;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LlmProvider {
    Openai,
    Anthropic,
    Gemini,
    OpenaiCompatible,
}

impl FromStr for LlmProvider {
    type Err = AppError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_ascii_lowercase().as_str() {
            "openai" => Ok(Self::Openai),
            "anthropic" | "claude" => Ok(Self::Anthropic),
            "gemini" => Ok(Self::Gemini),
            "openai-compatible" | "openai_compatible" | "relay" | "cc-switch" | "ccswitch" => {
                Ok(Self::OpenaiCompatible)
            }
            other => Err(AppError::InvalidConfig(format!(
                "unsupported provider: {other}. use openai|anthropic|gemini|openai-compatible"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub request_timeout_ms: u64,
    pub retry_max: u32,
    pub retry_base_ms: u64,
    pub max_ocr_chars: usize,
    pub anthropic_version: String,
    pub anthropic_max_tokens: u32,
}

impl RuntimeConfig {
    pub fn from_env() -> Self {
        Self {
            request_timeout_ms: env_u64("REQUEST_TIMEOUT_MS", 30_000),
            retry_max: env_u32("RETRY_MAX", 2),
            retry_base_ms: env_u64("RETRY_BASE_MS", 300),
            max_ocr_chars: env_usize("MAX_OCR_CHARS", 2_000_000),
            anthropic_version: std::env::var("ANTHROPIC_VERSION")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "2023-06-01".to_string()),
            anthropic_max_tokens: env_u32("ANTHROPIC_MAX_TOKENS", 4096),
        }
    }
}

pub fn env_u64(key: &str, fallback: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

pub fn env_u32(key: &str, fallback: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

pub fn env_usize(key: &str, fallback: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}
