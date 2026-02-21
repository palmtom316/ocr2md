use std::path::Path;

use ocr2md_core::file_kind::{InputKind, detect_input_kind};

#[test]
fn detects_pdf_kind() {
    let kind = detect_input_kind(Path::new("demo.pdf")).unwrap();
    assert_eq!(kind, InputKind::Pdf);
}
