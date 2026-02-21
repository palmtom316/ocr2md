use std::path::Path;

use anyhow::{Context, Result};
use tokio::fs;
use tracing::{info, warn};

use crate::config::RuntimeConfig;
use crate::http::HttpEngine;
use crate::llm::{LlmClient, LlmConfig};
use crate::ocr::{GlmConfig, GlmOcrClient};

pub async fn process_file(
    input_path: &Path,
    output_path: &Path,
    glm_cfg: GlmConfig,
    llm_cfg: LlmConfig,
    runtime: RuntimeConfig,
    trace_id: &str,
) -> Result<()> {
    info!(
        input = %input_path.display(),
        output = %output_path.display(),
        provider = ?llm_cfg.provider,
        trace_id,
        "pipeline_start"
    );

    let file_bytes = fs::read(input_path)
        .await
        .with_context(|| format!("failed to read input file: {}", input_path.display()))?;

    let http = HttpEngine::new(runtime.clone())?;

    info!(
        glm_base_url = %glm_cfg.base_url,
        glm_ocr_url = %glm_cfg.ocr_url,
        trace_id,
        "ocr_config_loaded"
    );

    let ocr_client = GlmOcrClient::new(http.clone(), glm_cfg);
    let ocr_text = ocr_client
        .extract_text(input_path, &file_bytes, trace_id)
        .await?;

    if ocr_text.trim().is_empty() {
        warn!(trace_id, "ocr_output_empty");
    }

    let llm_client = LlmClient::new(http, llm_cfg, runtime);
    let markdown = llm_client.to_markdown(&ocr_text, trace_id).await?;

    fs::write(output_path, markdown.as_bytes())
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
