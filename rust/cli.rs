use std::path::PathBuf;

use clap::Parser;
use ocr2md_core::config::LlmProvider;

#[derive(Debug, Parser)]
#[command(
    name = "ocr2md",
    version,
    about = "Cross-platform OCR to structured Markdown pipeline (Windows/macOS)"
)]
pub struct Cli {
    #[arg(value_name = "INPUT_FILE", help = "input file path (.pdf/.doc/.docx)")]
    pub input: PathBuf,

    #[arg(
        short,
        long,
        value_name = "OUTPUT_MD",
        help = "output markdown file path"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        long,
        value_enum,
        env = "LLM_PROVIDER",
        default_value = "openai-compatible",
        help = "commercial LLM provider"
    )]
    pub provider: LlmProvider,

    #[arg(long, env = "LLM_MODEL", help = "LLM model name")]
    pub llm_model: Option<String>,

    #[arg(
        long,
        env = "LLM_BASE_URL",
        help = "LLM API base URL (required for openai-compatible)"
    )]
    pub llm_base_url: Option<String>,

    #[arg(long, env = "LLM_API_KEY", help = "LLM API key")]
    pub llm_api_key: Option<String>,

    #[arg(long, env = "GLM_API_KEY", help = "GLM API key for OCR/file parsing")]
    pub glm_api_key: Option<String>,

    #[arg(long, env = "GLM_BASE_URL", help = "GLM API base URL")]
    pub glm_base_url: Option<String>,

    #[arg(long, env = "GLM_OCR_URL", help = "GLM OCR endpoint URL")]
    pub glm_ocr_url: Option<String>,

    #[arg(long, env = "GLM_FILE_PARSE_URL", help = "GLM file parse endpoint URL")]
    pub glm_file_parse_url: Option<String>,

    #[arg(long, env = "GLM_OCR_MODEL", help = "GLM OCR model name")]
    pub glm_ocr_model: Option<String>,

    #[arg(
        long,
        env = "SYSTEM_PROMPT",
        help = "override markdown structuring system prompt"
    )]
    pub system_prompt: Option<String>,

    #[arg(long, env = "TRACE_ID", help = "override trace id")]
    pub trace_id: Option<String>,
}
