# Ocr2Md (Rust)

跨平台（Windows / macOS）命令行程序：

1. 输入文件：`pdf`、`doc`、`docx`、双层 PDF（扫描层+文本层）。
2. OCR/解析阶段：
- `pdf`（含双层 PDF）走 `GLM-OCR`。
- `doc/docx` 走 GLM 文件解析接口（同一 GLM 平台能力）。
3. 结构化阶段：调用商业 AI API 生成结构化 Markdown，支持：
- OpenAI 官方
- Claude（Anthropic）官方
- Gemini 官方
- 任何 OpenAI-Compatible 中转站
- `cc-switch`（按 OpenAI-Compatible 配置）

## 项目结构

```text
.
├── Cargo.toml
├── rust
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── error.rs
│   ├── file_kind.rs
│   ├── http.rs
│   ├── llm.rs
│   └── ocr.rs
└── .env.example
```

## 安装与构建

```bash
# macOS / Windows (PowerShell 同理)
cargo build --release
```

可执行文件：
- macOS: `target/release/ocr2md`
- Windows: `target\\release\\ocr2md.exe`

## 配置

```bash
cp .env.example .env
```

最少必填：
- `GLM_API_KEY`
- `LLM_PROVIDER`
- `LLM_API_KEY`
- 若 `LLM_PROVIDER=openai-compatible`，还需 `LLM_BASE_URL`

## 用法

```bash
# 基本
cargo run -- /path/to/input.pdf --provider openai-compatible --output output.md

# OpenAI 官方
cargo run -- ./demo.pdf --provider openai --llm-api-key "$OPENAI_API_KEY" --llm-model gpt-4o-mini --llm-base-url https://api.openai.com/v1

# Claude 官方
cargo run -- ./demo.pdf --provider anthropic --llm-api-key "$ANTHROPIC_API_KEY" --llm-model claude-sonnet-4-5 --llm-base-url https://api.anthropic.com/v1

# Gemini 官方
cargo run -- ./demo.pdf --provider gemini --llm-api-key "$GEMINI_API_KEY" --llm-model gemini-2.0-flash --llm-base-url https://generativelanguage.googleapis.com/v1beta

# 中转站 / cc-switch（OpenAI-Compatible）
cargo run -- ./demo.pdf --provider openai-compatible --llm-base-url "https://your-relay-or-cc-switch.example/v1" --llm-api-key "$RELAY_KEY"
```

## 输出

默认输出路径：与输入同目录、同名 `.md`。
- 输入 `report.pdf` -> 输出 `report.md`

## 质量与验证

```bash
cargo fmt --all
cargo test
cargo build --release
```

## 设计说明

- 可靠性：HTTP 超时、429/5xx 重试、指数退避。
- 安全性：不记录原始文档内容，不记录 API Key。
- 扩展性：LLM provider 适配层可继续扩展（例如私有部署网关）。

## 注意事项

- `GLM-OCR` 主要用于 PDF 场景；Word 通过 GLM 文件解析能力补齐。
- 若你希望 Word 也强制转为 OCR 流程，可在后续版本接入本地 Office->PDF 转换链路。
