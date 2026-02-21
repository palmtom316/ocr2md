use std::path::Path;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};

use crate::error::AppError;
use crate::file_kind::{InputKind, detect_input_kind};
use crate::http::HttpEngine;

const DEFAULT_GLM_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";
const DEFAULT_GLM_OCR_MODEL: &str = "glm-4.1v-thinking-flashx";

#[derive(Debug, Clone)]
pub struct GlmConfig {
    pub api_key: String,
    pub base_url: String,
    pub ocr_model: String,
    pub ocr_url: String,
    pub file_parse_url: String,
    pub max_ocr_chars: usize,
}

impl GlmConfig {
    pub fn from_sources(
        api_key: Option<String>,
        base_url: Option<String>,
        ocr_model: Option<String>,
        ocr_url: Option<String>,
        file_parse_url: Option<String>,
        max_ocr_chars: usize,
    ) -> Result<Self> {
        let api_key = api_key
            .or_else(|| std::env::var("GLM_API_KEY").ok())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::InvalidConfig("GLM_API_KEY is required".to_string()))?;

        let base_url = base_url
            .or_else(|| std::env::var("GLM_BASE_URL").ok())
            .unwrap_or_else(|| DEFAULT_GLM_BASE_URL.to_string());
        let base_url = base_url.trim_end_matches('/').to_string();

        let ocr_model = ocr_model
            .or_else(|| std::env::var("GLM_OCR_MODEL").ok())
            .unwrap_or_else(|| DEFAULT_GLM_OCR_MODEL.to_string());

        let ocr_url = ocr_url
            .or_else(|| std::env::var("GLM_OCR_URL").ok())
            .unwrap_or_else(|| format!("{base_url}/chat/completions"));

        let file_parse_url = file_parse_url
            .or_else(|| std::env::var("GLM_FILE_PARSE_URL").ok())
            .unwrap_or_else(|| format!("{base_url}/files/parse"));

        Ok(Self {
            api_key,
            base_url,
            ocr_model,
            ocr_url,
            file_parse_url,
            max_ocr_chars,
        })
    }
}

pub struct GlmOcrClient {
    http: HttpEngine,
    cfg: GlmConfig,
}

impl GlmOcrClient {
    pub fn new(http: HttpEngine, cfg: GlmConfig) -> Self {
        Self { http, cfg }
    }

    pub async fn extract_text(
        &self,
        input_path: &Path,
        bytes: &[u8],
        trace_id: &str,
    ) -> Result<String> {
        match detect_input_kind(input_path)? {
            InputKind::Pdf => self.extract_pdf(input_path, bytes, trace_id).await,
            InputKind::Doc | InputKind::Docx => self.parse_word(input_path, bytes, trace_id).await,
        }
    }

    async fn extract_pdf(&self, input_path: &Path, bytes: &[u8], trace_id: &str) -> Result<String> {
        let mime = mime_guess::from_path(input_path)
            .first_raw()
            .unwrap_or("application/pdf");
        let data_url = format!("data:{mime};base64,{}", STANDARD.encode(bytes));

        let payload = json!({
            "model": self.cfg.ocr_model,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "file_url",
                            "file_url": {
                                "url": data_url
                            }
                        },
                        {
                            "type": "text",
                            "text": "请提取文档完整内容，尽量保留标题、段落和表格结构，输出纯文本。"
                        }
                    ]
                }
            ]
        });

        let response = self
            .http
            .post_json(
                "glm_ocr",
                &self.cfg.ocr_url,
                self.auth_headers()?,
                &payload,
                trace_id,
            )
            .await?;

        let text = parse_glm_ocr_text(&response)?;
        Ok(limit_text(text, self.cfg.max_ocr_chars))
    }

    async fn parse_word(&self, _input_path: &Path, bytes: &[u8], trace_id: &str) -> Result<String> {
        let payload = json!({
            "file": format!("base64://{}", STANDARD.encode(bytes)),
            "purpose": "file-extract",
            "prompt": "提取文档全部正文与结构信息，保留标题层级和表格文本。"
        });

        let response = self
            .http
            .post_json(
                "glm_file_parse",
                &self.cfg.file_parse_url,
                self.auth_headers()?,
                &payload,
                trace_id,
            )
            .await?;

        let text = parse_glm_file_parse_text(&response)?;
        Ok(limit_text(text, self.cfg.max_ocr_chars))
    }

    fn auth_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.cfg.api_key))
                .context("invalid GLM_API_KEY for header")?,
        );
        Ok(headers)
    }
}

fn limit_text(mut text: String, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text;
    }

    text = text.chars().take(max_chars).collect();
    text.push_str("\n\n[TRUNCATED: OCR output exceeded MAX_OCR_CHARS]");
    text
}

fn parse_glm_ocr_text(value: &Value) -> Result<String> {
    extract_openai_content(value).ok_or_else(|| {
        AppError::ApiResponse("missing choices[0].message.content in GLM OCR response".to_string())
            .into()
    })
}

fn parse_glm_file_parse_text(value: &Value) -> Result<String> {
    for pointer in [
        "/content",
        "/data/content",
        "/text",
        "/data/text",
        "/result/content",
    ] {
        if let Some(text) = value.pointer(pointer).and_then(Value::as_str)
            && !text.trim().is_empty()
        {
            return Ok(text.to_string());
        }
    }

    Err(
        AppError::ApiResponse("missing extracted text in GLM file parse response".to_string())
            .into(),
    )
}

pub fn extract_openai_content(value: &Value) -> Option<String> {
    let content = value.pointer("/choices/0/message/content")?;
    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }

    let mut buf = String::new();
    if let Some(parts) = content.as_array() {
        for part in parts {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(text);
            }
        }
    }

    if buf.trim().is_empty() {
        None
    } else {
        Some(buf)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::extract_openai_content;

    #[test]
    fn parse_openai_content_string() {
        let value = json!({
            "choices": [
                {
                    "message": {
                        "content": "hello"
                    }
                }
            ]
        });

        assert_eq!(extract_openai_content(&value).as_deref(), Some("hello"));
    }

    #[test]
    fn parse_openai_content_parts() {
        let value = json!({
            "choices": [
                {
                    "message": {
                        "content": [
                            {"type": "output_text", "text": "line1"},
                            {"type": "output_text", "text": "line2"}
                        ]
                    }
                }
            ]
        });

        assert_eq!(
            extract_openai_content(&value).as_deref(),
            Some("line1\nline2")
        );
    }
}
