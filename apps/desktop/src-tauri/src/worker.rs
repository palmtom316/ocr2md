use std::path::PathBuf;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

use ocr2md_core::config::{LlmProvider, RuntimeConfig};
use ocr2md_core::llm::LlmConfig;
use ocr2md_core::ocr::GlmConfig;
use ocr2md_core::pipeline::process_file;

use crate::state::AppState;

fn get_trace_id(job_id: u64) -> String {
    format!("job-{}", job_id)
}

pub fn spawn_worker(app_handle: AppHandle, state: AppState) {
    tokio::spawn(async move {
        loop {
            let job_id = {
                let queue = state.queue.lock().unwrap();
                queue.get_next_pending()
            };

            if let Some(id) = job_id {
                let (input_path_str, retries) = {
                    let mut queue = state.queue.lock().unwrap();
                    queue.mark_running(id, "starting");
                    let job = queue.get(id).unwrap();
                    (job.input.clone(), job.retries)
                };

                let _ = app_handle.emit("queue-updated", ());

                let input_path = PathBuf::from(&input_path_str);
                let output_path = resolve_output_path(&input_path);
                let trace_id = get_trace_id(id);

                let runtime = RuntimeConfig::from_env();

                let llm_cfg_opt = {
                    let profiles = state.active_profiles.lock().unwrap();
                    profiles.iter().find(|p| p.enabled).map(|p| {
                        let provider = match p.provider.as_str() {
                            "openai" => LlmProvider::Openai,
                            "anthropic" | "claude" => LlmProvider::Anthropic,
                            "gemini" => LlmProvider::Gemini,
                            _ => LlmProvider::OpenaiCompatible,
                        };
                        LlmConfig {
                            provider,
                            api_key: p.api_key.clone(),
                            base_url: p.base_url.clone(),
                            model: p.model.clone(),
                            system_prompt: std::env::var("SYSTEM_PROMPT").unwrap_or_else(|_| "你是一个严谨的文档结构化助手。将输入文本整理为高质量 Markdown，要求：\n1) 只输出 Markdown，不输出解释。\n2) 保留原文信息，不杜撰。\n3) 自动识别并组织标题层级、段落、列表、表格。\n4) 对明显噪声进行最小清洗（如重复页眉页脚）。\n5) 对公式、代码块、表格尽量保持可读性。".to_string()),
                        }
                    })
                };

                let glm_cfg_res = GlmConfig::from_sources(
                    std::env::var("GLM_API_KEY").ok(),
                    std::env::var("GLM_BASE_URL").ok(),
                    std::env::var("GLM_OCR_MODEL").ok(),
                    std::env::var("GLM_OCR_URL").ok(),
                    std::env::var("GLM_FILE_PARSE_URL").ok(),
                    runtime.max_ocr_chars,
                );

                if let Some(llm_cfg) = llm_cfg_opt {
                    if let Ok(glm_cfg) = glm_cfg_res {
                        {
                            let mut queue = state.queue.lock().unwrap();
                            queue.mark_running(id, "processing");
                        }
                        let _ = app_handle.emit("queue-updated", ());

                        match process_file(
                            &input_path,
                            &output_path,
                            glm_cfg,
                            llm_cfg,
                            runtime,
                            &trace_id,
                        )
                        .await
                        {
                            Ok(_) => {
                                let mut queue = state.queue.lock().unwrap();
                                queue.mark_success(id);
                            }
                            Err(e) => {
                                let mut queue = state.queue.lock().unwrap();
                                if retries < 3 {
                                    queue.mark_retrying(id, "failed_retry", e.to_string());
                                } else {
                                    queue.mark_failed(id, e.to_string());
                                }
                            }
                        }
                    } else {
                        let mut queue = state.queue.lock().unwrap();
                        queue.mark_failed(id, "GLM API Config missing (check env variables)");
                    }
                } else {
                    let mut queue = state.queue.lock().unwrap();
                    queue.mark_failed(
                        id,
                        "No active LLM profile found. Please load or configure a profile.",
                    );
                }

                let _ = app_handle.emit("queue-updated", ());
            } else {
                tokio::select! {
                    _ = state.notify_worker.notified() => {}
                    _ = sleep(Duration::from_secs(2)) => {}
                }
            }
        }
    });
}

fn resolve_output_path(input: &std::path::Path) -> PathBuf {
    if let Some(stem) = input.file_stem().and_then(|value| value.to_str()) {
        let mut path = input.to_path_buf();
        path.set_file_name(format!("{stem}.md"));
        path
    } else {
        PathBuf::from("output.md")
    }
}
