use ocr2md_core::queue::{JobState, Queue};

#[test]
fn job_state_transitions_to_success() {
    let mut q = Queue::default();
    let id = q.enqueue("demo.pdf");
    q.mark_running(id, "ocr");
    q.mark_running(id, "llm");
    q.mark_success(id);
    assert_eq!(q.get(id).unwrap().state, JobState::Success);
}
