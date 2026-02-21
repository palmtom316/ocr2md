use std::path::Path;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Pdf,
    Doc,
    Docx,
}

pub fn detect_input_kind(path: &Path) -> Result<InputKind, AppError> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| AppError::UnsupportedInputType(path.display().to_string()))?;

    match ext.as_str() {
        "pdf" => Ok(InputKind::Pdf),
        "doc" => Ok(InputKind::Doc),
        "docx" => Ok(InputKind::Docx),
        _ => Err(AppError::UnsupportedInputType(path.display().to_string())),
    }
}
