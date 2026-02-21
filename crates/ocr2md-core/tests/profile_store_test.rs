use ocr2md_core::profile_store::{ProfileStore, ProviderProfile};

#[test]
fn save_and_load_profiles() {
    let dir = tempfile::tempdir().unwrap();
    let store = ProfileStore::new(dir.path().join("config.enc"));
    let p = ProviderProfile::openai("work", "https://api.openai.com/v1", "k1", "gpt-4o-mini");
    store.save_all("pass", &[p]).unwrap();
    let loaded = store.load_all("pass").unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "work");
}
