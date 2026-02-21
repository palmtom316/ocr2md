mod cli;
mod config;
mod error;
mod file_kind;
mod http;
mod llm;
mod ocr;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::Parser;
use tokio::fs;
use tracing::{info, warn};

use crate::cli::Cli;
use crate::config::RuntimeConfig;
use crate::http::HttpEngine;
use crate::llm::{LlmClient, LlmConfig};
use crate::ocr::{GlmConfig, GlmOcrClient};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let cli = Cli::parse();
    let trace_id = cli.trace_id.unwrap_or_else(default_trace_id);

    let input_path = cli.input;
    let output_path = resolve_output_path(&input_path, cli.output);

    info!(
        input = %input_path.display(),
        output = %output_path.display(),
        provider = ?cli.provider,
        trace_id,
        "pipeline_start"
    );

    let runtime = RuntimeConfig::from_env();

    let file_bytes = fs::read(&input_path)
        .await
        .with_context(|| format!("failed to read input file: {}", input_path.display()))?;

    let http = HttpEngine::new(runtime.clone())?;

    let glm_cfg = GlmConfig::from_sources(
        cli.glm_api_key,
        cli.glm_base_url,
        cli.glm_ocr_model,
        cli.glm_ocr_url,
        cli.glm_file_parse_url,
        runtime.max_ocr_chars,
    )?;

    info!(
        glm_base_url = %glm_cfg.base_url,
        glm_ocr_url = %glm_cfg.ocr_url,
        trace_id,
        "ocr_config_loaded"
    );

    let ocr_client = GlmOcrClient::new(http.clone(), glm_cfg);
    let ocr_text = ocr_client
        .extract_text(&input_path, &file_bytes, &trace_id)
        .await?;

    if ocr_text.trim().is_empty() {
        warn!(trace_id, "ocr_output_empty");
    }

    let llm_cfg = LlmConfig::from_sources(
        cli.provider,
        cli.llm_api_key,
        cli.llm_base_url,
        cli.llm_model,
        cli.system_prompt,
    )?;

    let llm_client = LlmClient::new(http, llm_cfg, runtime);
    let markdown = llm_client.to_markdown(&ocr_text, &trace_id).await?;

    fs::write(&output_path, markdown.as_bytes())
        .await
        .with_context(|| format!("failed to write output: {}", output_path.display()))?;

    info!(
        output = %output_path.display(),
        bytes = markdown.len(),
        trace_id,
        "pipeline_done"
    );

    Ok(())
}

fn init_tracing() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

fn default_trace_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("trace-{ts}-{}", std::process::id())
}

fn resolve_output_path(input: &Path, output: Option<PathBuf>) -> PathBuf {
    if let Some(path) = output {
        return path;
    }

    if let Some(stem) = input.file_stem().and_then(|value| value.to_str()) {
        let mut path = input.to_path_buf();
        path.set_file_name(format!("{stem}.md"));
        path
    } else {
        PathBuf::from("output.md")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pretty_assertions::assert_eq;

    use crate::file_kind::{InputKind, detect_input_kind};

    use super::resolve_output_path;

    #[test]
    fn output_path_defaults_to_same_dir_md() {
        let input = Path::new("/tmp/demo.pdf");
        let out = resolve_output_path(input, None);
        assert_eq!(out.to_string_lossy(), "/tmp/demo.md");
    }

    #[test]
    fn detect_supported_kinds() {
        assert_eq!(
            detect_input_kind(Path::new("a.pdf")).ok(),
            Some(InputKind::Pdf)
        );
        assert_eq!(
            detect_input_kind(Path::new("b.doc")).ok(),
            Some(InputKind::Doc)
        );
        assert_eq!(
            detect_input_kind(Path::new("c.docx")).ok(),
            Some(InputKind::Docx)
        );
    }
}
