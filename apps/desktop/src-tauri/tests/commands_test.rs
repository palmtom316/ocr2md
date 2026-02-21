use ocr2md_desktop::{
    commands::{
        ProviderProfilePayload, enqueue_files_inner, load_profiles_inner, save_profiles_inner,
    },
    state::AppState,
};

#[tokio::test]
async fn enqueue_command_returns_job_id() {
    let state = AppState::default();
    let ids = enqueue_files_inner(&state, vec!["demo.pdf".to_string()]);
    assert!(!ids.is_empty());
}

#[tokio::test]
async fn saves_and_loads_profiles_with_passphrase() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");
    let state = AppState::for_profile_path(temp.path().join("profiles.enc"));
    let passphrase = "test-passphrase";
    let profiles = vec![ProviderProfilePayload {
        name: "Primary OpenAI".to_string(),
        provider: "openai".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: "sk-test".to_string(),
        model: "gpt-4.1-mini".to_string(),
        enabled: true,
    }];

    save_profiles_inner(&state, passphrase, profiles.clone()).expect("save failed");
    let loaded = load_profiles_inner(&state, passphrase).expect("load failed");

    assert_eq!(loaded, profiles);
}

#[tokio::test]
async fn rejects_empty_passphrase_for_profile_commands() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");
    let state = AppState::for_profile_path(temp.path().join("profiles.enc"));

    let save_error = save_profiles_inner(&state, "   ", Vec::new()).expect_err("save should fail");
    assert!(save_error.contains("passphrase"));

    let load_error = load_profiles_inner(&state, "").expect_err("load should fail");
    assert!(load_error.contains("passphrase"));
}
