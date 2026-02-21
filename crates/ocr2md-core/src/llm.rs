use anyhow::{Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};

use crate::config::{LlmProvider, RuntimeConfig};
use crate::error::AppError;
use crate::http::HttpEngine;
use crate::ocr::extract_openai_content;

const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";
const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub system_prompt: String,
}

impl LlmConfig {
    pub fn from_sources(
        provider: LlmProvider,
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
        system_prompt: Option<String>,
    ) -> Result<Self> {
        let api_key = api_key
            .or_else(|| std::env::var("LLM_API_KEY").ok())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::InvalidConfig("LLM_API_KEY is required".to_string()))?;

        let base_url = base_url
            .or_else(|| std::env::var("LLM_BASE_URL").ok())
            .unwrap_or_else(|| match provider {
                LlmProvider::Openai => DEFAULT_OPENAI_BASE_URL.to_string(),
                LlmProvider::Anthropic => DEFAULT_ANTHROPIC_BASE_URL.to_string(),
                LlmProvider::Gemini => DEFAULT_GEMINI_BASE_URL.to_string(),
                LlmProvider::OpenaiCompatible => String::new(),
            });

        if provider == LlmProvider::OpenaiCompatible && base_url.trim().is_empty() {
            return Err(AppError::InvalidConfig(
                "LLM_BASE_URL is required for openai-compatible/relay/cc-switch provider"
                    .to_string(),
            )
            .into());
        }

        let model = model
            .or_else(|| std::env::var("LLM_MODEL").ok())
            .unwrap_or_else(|| match provider {
                LlmProvider::Openai => "gpt-4o-mini".to_string(),
                LlmProvider::Anthropic => "claude-sonnet-4-5".to_string(),
                LlmProvider::Gemini => "gemini-2.0-flash".to_string(),
                LlmProvider::OpenaiCompatible => "gpt-4o-mini".to_string(),
            });

        let system_prompt = system_prompt.unwrap_or_else(default_system_prompt);

        Ok(Self {
            provider,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            system_prompt,
        })
    }
}

pub struct LlmClient {
    http: HttpEngine,
    cfg: LlmConfig,
    runtime: RuntimeConfig,
}

impl LlmClient {
    pub fn new(http: HttpEngine, cfg: LlmConfig, runtime: RuntimeConfig) -> Self {
        Self { http, cfg, runtime }
    }

    pub async fn to_markdown(&self, ocr_text: &str, trace_id: &str) -> Result<String> {
        let user_prompt = build_user_prompt(ocr_text);

        match self.cfg.provider {
            LlmProvider::Openai | LlmProvider::OpenaiCompatible => {
                self.call_openai_compatible(&user_prompt, trace_id).await
            }
            LlmProvider::Anthropic => self.call_anthropic(&user_prompt, trace_id).await,
            LlmProvider::Gemini => self.call_gemini(&user_prompt, trace_id).await,
        }
    }

    async fn call_openai_compatible(&self, user_prompt: &str, trace_id: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.cfg.base_url);

        let payload = json!({
            "model": self.cfg.model,
            "temperature": 0.1,
            "messages": [
                {
                    "role": "system",
                    "content": self.cfg.system_prompt
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ]
        });

        let response = self
            .http
            .post_json(
                "llm_openai_compatible",
                &url,
                bearer_headers(&self.cfg.api_key)?,
                &payload,
                trace_id,
            )
            .await?;

        extract_openai_content(&response)
            .ok_or_else(|| AppError::ApiResponse("missing OpenAI content".to_string()).into())
    }

    async fn call_anthropic(&self, user_prompt: &str, trace_id: &str) -> Result<String> {
        let url = format!("{}/messages", self.cfg.base_url);

        let payload = json!({
            "model": self.cfg.model,
            "max_tokens": self.runtime.anthropic_max_tokens,
            "system": self.cfg.system_prompt,
            "messages": [
                {
                    "role": "user",
                    "content": user_prompt
                }
            ]
        });

        let response = self
            .http
            .post_json(
                "llm_anthropic",
                &url,
                anthropic_headers(&self.cfg.api_key, &self.runtime.anthropic_version)?,
                &payload,
                trace_id,
            )
            .await?;

        parse_anthropic_content(&response)
            .ok_or_else(|| AppError::ApiResponse("missing Anthropic content".to_string()).into())
    }

    async fn call_gemini(&self, user_prompt: &str, trace_id: &str) -> Result<String> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.cfg.base_url, self.cfg.model, self.cfg.api_key
        );

        let merged_prompt = format!("{}\n\n{}", self.cfg.system_prompt, user_prompt);
        let payload = json!({
            "contents": [
                {
                    "role": "user",
                    "parts": [
                        {
                            "text": merged_prompt
                        }
                    ]
                }
            ],
            "generationConfig": {
                "temperature": 0.1
            }
        });

        let response = self
            .http
            .post_json("llm_gemini", &url, json_headers()?, &payload, trace_id)
            .await?;

        parse_gemini_content(&response)
            .ok_or_else(|| AppError::ApiResponse("missing Gemini content".to_string()).into())
    }
}

fn default_system_prompt() -> String {
    "你是一个严谨的文档结构化助手。将输入文本整理为高质量 Markdown，要求：\n1) 只输出 Markdown，不输出解释。\n2) 保留原文信息，不杜撰。\n3) 自动识别并组织标题层级、段落、列表、表格。\n4) 对明显噪声进行最小清洗（如重复页眉页脚）。\n5) 对公式、代码块、表格尽量保持可读性。"
        .to_string()
}

fn build_user_prompt(ocr_text: &str) -> String {
    format!(
        "请将下面 OCR 文本整理成结构化 Markdown。\n\n--- OCR START ---\n{}\n--- OCR END ---",
        ocr_text
    )
}

fn bearer_headers(api_key: &str) -> Result<HeaderMap> {
    let mut headers = json_headers()?;
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {api_key}"))
            .context("invalid LLM_API_KEY for bearer header")?,
    );
    Ok(headers)
}

fn anthropic_headers(api_key: &str, version: &str) -> Result<HeaderMap> {
    let mut headers = json_headers()?;
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).context("invalid LLM_API_KEY for anthropic header")?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_str(version).context("invalid ANTHROPIC_VERSION")?,
    );
    Ok(headers)
}

fn json_headers() -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(headers)
}

pub fn parse_anthropic_content(value: &Value) -> Option<String> {
    let content = value.pointer("/content")?.as_array()?;
    let mut out = String::new();

    for item in content {
        if let Some(text) = item.get("text").and_then(Value::as_str) {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text);
        }
    }

    if out.trim().is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn parse_gemini_content(value: &Value) -> Option<String> {
    let parts = value.pointer("/candidates/0/content/parts")?.as_array()?;
    let mut out = String::new();

    for part in parts {
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text);
        }
    }

    if out.trim().is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::{parse_anthropic_content, parse_gemini_content};

    #[test]
    fn parse_anthropic_response() {
        let value = json!({
            "content": [
                {"type": "text", "text": "# Title"},
                {"type": "text", "text": "body"}
            ]
        });

        assert_eq!(
            parse_anthropic_content(&value).as_deref(),
            Some("# Title\nbody")
        );
    }

    #[test]
    fn parse_gemini_response() {
        let value = json!({
            "candidates": [
                {
                    "content": {
                        "parts": [
                            {"text": "# Title"},
                            {"text": "body"}
                        ]
                    }
                }
            ]
        });

        assert_eq!(
            parse_gemini_content(&value).as_deref(),
            Some("# Title\nbody")
        );
    }
}
