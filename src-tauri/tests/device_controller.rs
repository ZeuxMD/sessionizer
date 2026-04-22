use sessionizer_lib::device_controller::{AdminSessionState, DeviceController};
use sessionizer_lib::state_store::StateStore;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock drifted backwards")
        .as_nanos();

    std::env::temp_dir().join(format!("sessionizer-{label}-{unique}"))
}

#[test]
fn locked_device_stays_locked_after_legacy_profile_config_is_deleted() {
    let legacy_root = unique_dir("legacy-root");
    let machine_root = unique_dir("machine-root");

    let legacy_controller = DeviceController::new(StateStore::legacy_profile(legacy_root.clone()));
    let recovery_key = legacy_controller
        .setup_password("parent-secret".to_string(), 60)
        .expect("setup should succeed");
    assert!(!recovery_key.is_empty());
    legacy_controller
        .finish_setup()
        .expect("finishing setup should succeed");
    legacy_controller.relock().expect("relock should succeed");

    let migrated_controller = DeviceController::new(StateStore::new(
        machine_root.clone(),
        Some(legacy_root.clone()),
    ));

    let snapshot = migrated_controller
        .snapshot()
        .expect("snapshot should load");
    assert!(matches!(snapshot.session_state, AdminSessionState::Locked));

    let legacy_file = legacy_root.join("config.json");
    assert!(
        legacy_file.exists(),
        "legacy state should exist before deletion"
    );
    fs::remove_file(&legacy_file).expect("legacy state should be deletable");

    let snapshot = migrated_controller
        .snapshot()
        .expect("machine-scoped snapshot should still load");
    assert!(matches!(snapshot.session_state, AdminSessionState::Locked));

    let machine_file = machine_root.join("device-state.json");
    assert!(
        machine_file.exists(),
        "machine-scoped state should be persisted"
    );

    let _ = fs::remove_dir_all(legacy_root);
    let _ = fs::remove_dir_all(machine_root);
}
