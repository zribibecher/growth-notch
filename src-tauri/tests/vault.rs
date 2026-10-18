mod common;

use common::TempDir;
use growth_notch_lib::credentials::{CredentialStatus, FileVault};
use growth_notch_lib::security::CredentialProvider::{YoutubeApiKey, YoutubeChannelId};

const KEY: &str = "AIzaSyVaultTestKey_777888999";

fn secret_of(
    vault: &FileVault,
    provider: growth_notch_lib::security::CredentialProvider,
) -> Option<String> {
    vault
        .get_secret(provider)
        .unwrap()
        .map(|secret| secret.expose_secret().to_string())
}

#[test]
fn secrets_are_encrypted_on_disk() {
    let dir = TempDir::new("encrypted");
    let file = dir.join("vault.enc");
    let vault = FileVault::open(file.clone());
    vault.set_secret(YoutubeApiKey, KEY).unwrap();
    assert_eq!(secret_of(&vault, YoutubeApiKey).as_deref(), Some(KEY));

    assert_eq!(std::fs::read(dir.join(".vault.key")).unwrap().len(), 32);
    let on_disk = std::fs::read(&file).unwrap();
    assert!(
        on_disk.len() > 12 + 16,
        "nonce and authentication tag are missing"
    );
    let text = String::from_utf8_lossy(&on_disk);
    assert!(!text.contains(KEY), "the key is stored in plain text");
    assert!(
        !text.contains("youtube"),
        "field names are stored in plain text"
    );
}

#[test]
fn a_restart_reads_back_what_was_saved_and_deletes_stick() {
    let dir = TempDir::new("restart");
    let file = dir.join("vault.enc");
    let vault = FileVault::open(file.clone());
    vault
        .set_secret(YoutubeChannelId, "UC_COLD_BOOT_01")
        .unwrap();
    vault.set_secret(YoutubeApiKey, KEY).unwrap();

    let restarted = FileVault::open(file.clone());
    assert_eq!(
        secret_of(&restarted, YoutubeChannelId).as_deref(),
        Some("UC_COLD_BOOT_01")
    );
    assert_eq!(secret_of(&restarted, YoutubeApiKey).as_deref(), Some(KEY));

    restarted.delete_secret(YoutubeApiKey).unwrap();
    let after_delete = FileVault::open(file);
    assert_eq!(secret_of(&after_delete, YoutubeApiKey), None);
    assert!(secret_of(&after_delete, YoutubeChannelId).is_some());
}

#[test]
fn a_tampered_or_truncated_vault_opens_empty() {
    let dir = TempDir::new("tamper");
    let file = dir.join("vault.enc");
    FileVault::open(file.clone())
        .set_secret(YoutubeApiKey, KEY)
        .unwrap();

    // Flip a bit in the authentication tag.
    let mut bytes = std::fs::read(&file).unwrap();
    *bytes.last_mut().unwrap() ^= 0xFF;
    std::fs::write(&file, &bytes).unwrap();
    assert_eq!(
        secret_of(&FileVault::open(file.clone()), YoutubeApiKey),
        None
    );

    // Shorter than the nonce.
    std::fs::write(&file, [1u8; 8]).unwrap();
    assert_eq!(secret_of(&FileVault::open(file), YoutubeApiKey), None);
}

#[test]
fn losing_the_key_file_means_starting_over_not_crashing() {
    let dir = TempDir::new("lost-key");
    let file = dir.join("vault.enc");
    FileVault::open(file.clone())
        .set_secret(YoutubeApiKey, KEY)
        .unwrap();
    std::fs::remove_file(dir.join(".vault.key")).unwrap();

    let vault = FileVault::open(file.clone());
    assert_eq!(secret_of(&vault, YoutubeApiKey), None);
    vault
        .set_secret(YoutubeApiKey, "AIzaSyNewKeyAfterReset_123")
        .unwrap();
    assert_eq!(
        secret_of(&FileVault::open(file), YoutubeApiKey).as_deref(),
        Some("AIzaSyNewKeyAfterReset_123")
    );
}

#[cfg(unix)]
#[test]
fn only_the_owner_can_read_the_vault() {
    use std::os::unix::fs::PermissionsExt;
    let mode =
        |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;

    let dir = TempDir::new("perms");
    // A directory left over from an older version, readable by everyone.
    std::fs::create_dir_all(dir.path()).unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

    FileVault::open(dir.join("vault.enc"))
        .set_secret(YoutubeApiKey, KEY)
        .unwrap();
    assert_eq!(mode(dir.path()), 0o700, "data directory");
    assert_eq!(mode(&dir.join("vault.enc")), 0o600, "vault.enc");
    assert_eq!(mode(&dir.join(".vault.key")), 0o600, ".vault.key");
}

#[test]
fn saving_leaves_no_temporary_files_behind() {
    let dir = TempDir::new("atomic");
    let vault = FileVault::open(dir.join("vault.enc"));
    vault.set_secret(YoutubeApiKey, KEY).unwrap();
    vault
        .set_secret(YoutubeChannelId, "UC_ATOMIC_CHANNEL_ID_0001")
        .unwrap();

    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, [".vault.key", "vault.enc"]);
}

#[test]
fn status_says_what_is_set_and_nothing_else() {
    let vault = FileVault::in_memory();
    assert_eq!(vault.status().unwrap(), CredentialStatus::default());

    vault.set_secret(YoutubeApiKey, KEY).unwrap();
    let status = vault.status().unwrap();
    assert!(status.youtube_configured && !status.youtube_channel_configured);

    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(
        json,
        r#"{"youtubeConfigured":true,"youtubeChannelConfigured":false}"#
    );
}

#[test]
fn values_that_could_change_a_request_are_refused() {
    let vault = FileVault::in_memory();
    for bad_key in [
        "",
        "   ",
        "AIzaSyKey&evil=true",
        "AIza?x=1",
        "AIza#frag",
        "AIza/../x",
        "AIza with space",
        &"A".repeat(257),
    ] {
        assert!(
            vault.set_secret(YoutubeApiKey, bad_key).is_err(),
            "{bad_key:?}"
        );
    }
    for bad_id in ["", "../../v1/admin", "UC123?alt=media", "UC 123"] {
        assert!(
            vault.set_secret(YoutubeChannelId, bad_id).is_err(),
            "{bad_id:?}"
        );
    }
    assert_eq!(vault.status().unwrap(), CredentialStatus::default());
}

#[test]
fn an_in_memory_vault_writes_nothing_to_disk() {
    let vault = FileVault::in_memory();
    vault.set_secret(YoutubeApiKey, KEY).unwrap();
    assert_eq!(secret_of(&vault, YoutubeApiKey).as_deref(), Some(KEY));
}
