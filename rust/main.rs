mod cli;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use clap::Parser;
use ocr2md_core::config::RuntimeConfig;
use ocr2md_core::llm::LlmConfig;
use ocr2md_core::ocr::GlmConfig;
use ocr2md_core::pipeline::process_file;

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let cli = Cli::parse();
    let trace_id = cli.trace_id.unwrap_or_else(default_trace_id);

    let input_path = cli.input;
    let output_path = resolve_output_path(&input_path, cli.output);

    let runtime = RuntimeConfig::from_env();

    let glm_cfg = GlmConfig::from_sources(
        cli.glm_api_key,
        cli.glm_base_url,
        cli.glm_ocr_model,
        cli.glm_ocr_url,
        cli.glm_file_parse_url,
        runtime.max_ocr_chars,
    )?;

    let llm_cfg = LlmConfig::from_sources(
        cli.provider,
        cli.llm_api_key,
        cli.llm_base_url,
        cli.llm_model,
        cli.system_prompt,
    )?;

    process_file(
        &input_path,
        &output_path,
        glm_cfg,
        llm_cfg,
        runtime,
        &trace_id,
    )
    .await?;

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

    use super::resolve_output_path;
    use ocr2md_core::file_kind::{InputKind, detect_input_kind};
    use pretty_assertions::assert_eq;

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
