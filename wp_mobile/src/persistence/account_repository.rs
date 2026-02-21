use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use zeroize::Zeroize;

pub type AccountId = u64;

uniffi::custom_newtype!(EncryptedPassword, String);
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedPassword(pub String);

uniffi::custom_type!(DecryptedPassword, String, {
    lower: |obj| obj.0.clone(),
    try_lift: |val| Ok(DecryptedPassword(val)),
});
#[derive(Clone)]
pub struct DecryptedPassword(pub String);

impl Drop for DecryptedPassword {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for DecryptedPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DecryptedPassword(***)")
    }
}

#[uniffi::export(with_foreign)]
pub trait PasswordTransformer: Send + Sync {
    fn encrypt(
        &self,
        password: DecryptedPassword,
    ) -> Result<EncryptedPassword, PasswordTransformerError>;
    fn decrypt(
        &self,
        password: EncryptedPassword,
    ) -> Result<DecryptedPassword, PasswordTransformerError>;
}

/// The caller-facing account type. Passwords are always decrypted —
/// `AccountRepository` handles encryption/decryption transparently.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum Account {
    WpCom {
        id: AccountId,
        username: String,
        token: DecryptedPassword,
        site_api_root: String,
    },
    SelfHostedSite {
        id: AccountId,
        domain: String,
        username: String,
        password: DecryptedPassword,
        site_api_root: String,
    },
}

#[uniffi::export]
impl Account {
    pub fn id(&self) -> AccountId {
        match self {
            Account::WpCom { id, .. } => *id,
            Account::SelfHostedSite { id, .. } => *id,
        }
    }

    pub fn is_wp_com(&self) -> bool {
        matches!(self, Account::WpCom { .. })
    }

    pub fn is_self_hosted(&self) -> bool {
        matches!(self, Account::SelfHostedSite { .. })
    }
}

/// Internal storage representation with encrypted passwords.
/// Variant names match `Account` so existing JSON files deserialize correctly.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum StoredAccount {
    WpCom {
        id: AccountId,
        username: String,
        token: EncryptedPassword,
        site_api_root: String,
    },
    SelfHostedSite {
        id: AccountId,
        domain: String,
        username: String,
        password: EncryptedPassword,
        site_api_root: String,
    },
}

impl StoredAccount {
    fn id(&self) -> AccountId {
        match self {
            StoredAccount::WpCom { id, .. } => *id,
            StoredAccount::SelfHostedSite { id, .. } => *id,
        }
    }

    fn with_id(self, new_id: AccountId) -> Self {
        match self {
            StoredAccount::WpCom {
                username,
                token,
                site_api_root,
                ..
            } => StoredAccount::WpCom {
                id: new_id,
                username,
                token,
                site_api_root,
            },
            StoredAccount::SelfHostedSite {
                domain,
                username,
                password,
                site_api_root,
                ..
            } => StoredAccount::SelfHostedSite {
                id: new_id,
                domain,
                username,
                password,
                site_api_root,
            },
        }
    }

    fn encrypt(
        account: Account,
        transformer: &dyn PasswordTransformer,
    ) -> Result<Self, PasswordTransformerError> {
        match account {
            Account::WpCom {
                id,
                username,
                token,
                site_api_root,
            } => Ok(StoredAccount::WpCom {
                id,
                username,
                token: transformer.encrypt(token)?,
                site_api_root,
            }),
            Account::SelfHostedSite {
                id,
                domain,
                username,
                password,
                site_api_root,
            } => Ok(StoredAccount::SelfHostedSite {
                id,
                domain,
                username,
                password: transformer.encrypt(password)?,
                site_api_root,
            }),
        }
    }

    fn decrypt(
        &self,
        transformer: &dyn PasswordTransformer,
    ) -> Result<Account, PasswordTransformerError> {
        match self {
            StoredAccount::WpCom {
                id,
                username,
                token,
                site_api_root,
            } => Ok(Account::WpCom {
                id: *id,
                username: username.clone(),
                token: transformer.decrypt(token.clone())?,
                site_api_root: site_api_root.clone(),
            }),
            StoredAccount::SelfHostedSite {
                id,
                domain,
                username,
                password,
                site_api_root,
            } => Ok(Account::SelfHostedSite {
                id: *id,
                domain: domain.clone(),
                username: username.clone(),
                password: transformer.decrypt(password.clone())?,
                site_api_root: site_api_root.clone(),
            }),
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PasswordTransformerError {
    #[error("Encryption failed: {reason}")]
    EncryptionFailed { reason: String },
    #[error("Decryption failed: {reason}")]
    DecryptionFailed { reason: String },
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum AccountRepositoryError {
    #[error("IO error: {reason}")]
    IoError { reason: String },
    #[error("Password error: {reason}")]
    PasswordError { reason: String },
}

impl From<PasswordTransformerError> for AccountRepositoryError {
    fn from(e: PasswordTransformerError) -> Self {
        AccountRepositoryError::PasswordError {
            reason: e.to_string(),
        }
    }
}

#[derive(uniffi::Object)]
pub struct AccountRepository {
    file_path: PathBuf,
    accounts: Mutex<Vec<StoredAccount>>,
    next_id: Mutex<AccountId>,
    password_transformer: Arc<dyn PasswordTransformer>,
}

impl AccountRepository {
    fn save(&self, accounts: &[StoredAccount]) -> Result<(), AccountRepositoryError> {
        let json = serde_json::to_string_pretty(accounts).map_err(|e| {
            AccountRepositoryError::IoError {
                reason: e.to_string(),
            }
        })?;

        // Atomic write: write to a temp file then rename, so a crash can never
        // leave a half-written accounts.json.
        let temp_path = self.file_path.with_extension("tmp");
        fs::write(&temp_path, json).map_err(|e| AccountRepositoryError::IoError {
            reason: e.to_string(),
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600)).map_err(|e| {
                AccountRepositoryError::IoError {
                    reason: e.to_string(),
                }
            })?;
        }

        fs::rename(&temp_path, &self.file_path).map_err(|e| AccountRepositoryError::IoError {
            reason: e.to_string(),
        })
    }
}

#[uniffi::export]
impl AccountRepository {
    #[uniffi::constructor]
    pub fn new(
        root_path: String,
        password_transformer: Arc<dyn PasswordTransformer>,
    ) -> Result<Self, AccountRepositoryError> {
        let root = PathBuf::from(&root_path);

        fs::create_dir_all(&root).map_err(|e| AccountRepositoryError::IoError {
            reason: e.to_string(),
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).map_err(|e| {
                AccountRepositoryError::IoError {
                    reason: e.to_string(),
                }
            })?;
        }

        let file_path = root.join("accounts.json");

        let accounts: Vec<StoredAccount> = if file_path.exists() {
            let data =
                fs::read_to_string(&file_path).map_err(|e| AccountRepositoryError::IoError {
                    reason: e.to_string(),
                })?;
            serde_json::from_str(&data).map_err(|e| AccountRepositoryError::IoError {
                reason: e.to_string(),
            })?
        } else {
            Vec::new()
        };

        let next_id = accounts.iter().map(|a| a.id()).max().unwrap_or(0) + 1;

        Ok(Self {
            file_path,
            accounts: Mutex::new(accounts),
            next_id: Mutex::new(next_id),
            password_transformer,
        })
    }

    pub fn store(&self, account: Account) -> Result<AccountId, AccountRepositoryError> {
        let mut accounts = self.accounts.lock().expect("poisoned mutex");
        let mut next_id = self.next_id.lock().expect("poisoned mutex");

        let id = *next_id;
        *next_id += 1;

        let stored =
            StoredAccount::encrypt(account, self.password_transformer.as_ref())?.with_id(id);
        accounts.push(stored);
        self.save(&accounts)?;
        Ok(id)
    }

    pub fn all(&self) -> Result<Vec<Account>, AccountRepositoryError> {
        let accounts = self.accounts.lock().expect("poisoned mutex");
        accounts
            .iter()
            .map(|a| {
                a.decrypt(self.password_transformer.as_ref())
                    .map_err(Into::into)
            })
            .collect()
    }

    pub fn has_wp_com_account(&self) -> bool {
        self.accounts
            .lock()
            .expect("poisoned mutex")
            .iter()
            .any(|a| matches!(a, StoredAccount::WpCom { .. }))
    }

    pub fn has_self_hosted_account(&self) -> bool {
        self.accounts
            .lock()
            .expect("poisoned mutex")
            .iter()
            .any(|a| matches!(a, StoredAccount::SelfHostedSite { .. }))
    }

    pub fn get(&self, id: AccountId) -> Result<Option<Account>, AccountRepositoryError> {
        let accounts = self.accounts.lock().expect("poisoned mutex");
        match accounts.iter().find(|a| a.id() == id) {
            Some(stored) => Ok(Some(
                stored
                    .decrypt(self.password_transformer.as_ref())
                    .map_err(AccountRepositoryError::from)?,
            )),
            None => Ok(None),
        }
    }

    pub fn remove(&self, id: AccountId) -> Result<(), AccountRepositoryError> {
        let mut accounts = self.accounts.lock().expect("poisoned mutex");
        accounts.retain(|a| a.id() != id);
        self.save(&accounts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_transformer() -> Arc<dyn PasswordTransformer> {
        Arc::new(
            super::super::aes_gcm_transformer::AesGcmPasswordTransformer::new(
                "test-secret".to_string(),
            ),
        )
    }

    fn test_wp_com_account(domain: &str) -> Account {
        Account::WpCom {
            id: 0,
            username: format!("user@{domain}"),
            token: DecryptedPassword("secret".to_string()),
            site_api_root: format!("https://{domain}/wp-json"),
        }
    }

    fn test_self_hosted_account(domain: &str) -> Account {
        Account::SelfHostedSite {
            id: 0,
            domain: domain.to_string(),
            username: format!("user@{domain}"),
            password: DecryptedPassword("secret".to_string()),
            site_api_root: format!("https://{domain}/wp-json"),
        }
    }

    fn temp_repo() -> (tempfile::TempDir, AccountRepository) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let repo =
            AccountRepository::new(dir.path().to_string_lossy().to_string(), test_transformer())
                .expect("failed to create repo");
        (dir, repo)
    }

    #[test]
    fn test_new_creates_directory() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let nested = dir.path().join("a").join("b").join("c");
        assert!(!nested.exists());

        let _repo =
            AccountRepository::new(nested.to_string_lossy().to_string(), test_transformer())
                .expect("failed to create repo");

        assert!(nested.exists());
    }

    #[test]
    fn test_get_returns_none_for_unknown_id() {
        let (_dir, repo) = temp_repo();
        assert!(repo.get(999).expect("get failed").is_none());
    }

    #[test]
    fn test_store_and_get_wp_com() {
        let (_dir, repo) = temp_repo();
        let id = repo
            .store(test_wp_com_account("example.com"))
            .expect("store failed");

        let retrieved = repo.get(id).expect("get failed").expect("account missing");
        match retrieved {
            Account::WpCom {
                id: rid,
                username,
                token,
                ..
            } => {
                assert_eq!(rid, id);
                assert_eq!(username, "user@example.com");
                assert_eq!(token.0, "secret");
            }
            _ => panic!("expected WpCom variant"),
        }
    }

    #[test]
    fn test_store_and_get_self_hosted() {
        let (_dir, repo) = temp_repo();
        let id = repo
            .store(test_self_hosted_account("example.com"))
            .expect("store failed");

        let retrieved = repo.get(id).expect("get failed").expect("account missing");
        match retrieved {
            Account::SelfHostedSite {
                id: rid,
                domain,
                username,
                password,
                ..
            } => {
                assert_eq!(rid, id);
                assert_eq!(domain, "example.com");
                assert_eq!(username, "user@example.com");
                assert_eq!(password.0, "secret");
            }
            _ => panic!("expected SelfHostedSite variant"),
        }
    }

    #[test]
    fn test_store_assigns_incrementing_ids() {
        let (_dir, repo) = temp_repo();

        let id1 = repo
            .store(test_wp_com_account("a.com"))
            .expect("store failed");
        let id2 = repo
            .store(test_self_hosted_account("b.com"))
            .expect("store failed");
        let id3 = repo
            .store(test_wp_com_account("c.com"))
            .expect("store failed");

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_store_multiple_accounts() {
        let (_dir, repo) = temp_repo();

        let id1 = repo
            .store(test_wp_com_account("a.com"))
            .expect("store failed");
        let id2 = repo
            .store(test_self_hosted_account("b.com"))
            .expect("store failed");

        assert!(repo.get(id1).expect("get failed").is_some());
        assert!(repo.get(id2).expect("get failed").is_some());
        assert!(repo.get(999).expect("get failed").is_none());
    }

    #[test]
    fn test_remove_existing_account() {
        let (_dir, repo) = temp_repo();

        let id = repo
            .store(test_wp_com_account("example.com"))
            .expect("store failed");
        repo.remove(id).expect("remove failed");

        assert!(repo.get(id).expect("get failed").is_none());
    }

    #[test]
    fn test_remove_nonexistent_account_is_noop() {
        let (_dir, repo) = temp_repo();
        repo.remove(999)
            .expect("remove should succeed even if account doesn't exist");
    }

    #[test]
    fn test_remove_only_affects_matching_id() {
        let (_dir, repo) = temp_repo();

        let id1 = repo
            .store(test_wp_com_account("a.com"))
            .expect("store failed");
        let id2 = repo
            .store(test_self_hosted_account("b.com"))
            .expect("store failed");

        repo.remove(id1).expect("remove failed");

        assert!(repo.get(id1).expect("get failed").is_none());
        assert!(repo.get(id2).expect("get failed").is_some());
    }

    #[test]
    fn test_data_persists_across_instances() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().to_string_lossy().to_string();
        let transformer = test_transformer();

        let id = {
            let repo = AccountRepository::new(path.clone(), Arc::clone(&transformer))
                .expect("failed to create repo");
            repo.store(test_self_hosted_account("example.com"))
                .expect("store failed")
        };

        let repo = AccountRepository::new(path, transformer).expect("failed to create repo");
        let retrieved = repo.get(id).expect("get failed").expect("account missing");
        assert_eq!(retrieved.id(), id);
    }

    #[test]
    fn test_ids_continue_incrementing_across_instances() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().to_string_lossy().to_string();
        let transformer = test_transformer();

        let id1 = {
            let repo = AccountRepository::new(path.clone(), Arc::clone(&transformer))
                .expect("failed to create repo");
            repo.store(test_wp_com_account("a.com"))
                .expect("store failed")
        };

        let repo = AccountRepository::new(path, transformer).expect("failed to create repo");
        let id2 = repo
            .store(test_self_hosted_account("b.com"))
            .expect("store failed");

        assert!(id2 > id1);
    }

    #[test]
    fn test_remove_persists_across_instances() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().to_string_lossy().to_string();
        let transformer = test_transformer();

        let (id1, id2) = {
            let repo = AccountRepository::new(path.clone(), Arc::clone(&transformer))
                .expect("failed to create repo");
            let id1 = repo
                .store(test_wp_com_account("a.com"))
                .expect("store failed");
            let id2 = repo
                .store(test_self_hosted_account("b.com"))
                .expect("store failed");
            repo.remove(id1).expect("remove failed");
            (id1, id2)
        };

        let repo = AccountRepository::new(path, transformer).expect("failed to create repo");
        assert!(repo.get(id1).expect("get failed").is_none());
        assert!(repo.get(id2).expect("get failed").is_some());
    }

    #[test]
    fn test_mixed_variants_coexist() {
        let (_dir, repo) = temp_repo();

        let wp_id = repo
            .store(test_wp_com_account("wp.example.com"))
            .expect("store failed");
        let sh_id = repo
            .store(test_self_hosted_account("self.example.com"))
            .expect("store failed");

        assert!(matches!(
            repo.get(wp_id).expect("get failed"),
            Some(Account::WpCom { .. })
        ));
        assert!(matches!(
            repo.get(sh_id).expect("get failed"),
            Some(Account::SelfHostedSite { .. })
        ));
    }

    #[test]
    fn test_concurrent_stores() {
        let (_dir, repo) = temp_repo();
        let repo = Arc::new(repo);

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let repo = Arc::clone(&repo);
                std::thread::spawn(move || {
                    repo.store(test_wp_com_account(&format!("{i}.com")))
                        .expect("store failed")
                })
            })
            .collect();

        let ids: Vec<AccountId> = handles
            .into_iter()
            .map(|h| h.join().expect("thread panicked"))
            .collect();

        for id in &ids {
            assert!(
                repo.get(*id).expect("get failed").is_some(),
                "account with id {id} missing"
            );
        }

        // All IDs should be unique
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "IDs should be unique");
    }

    #[test]
    fn test_concurrent_stores_persist() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().to_string_lossy().to_string();
        let transformer = test_transformer();

        let ids = {
            let repo = Arc::new(
                AccountRepository::new(path.clone(), Arc::clone(&transformer))
                    .expect("failed to create repo"),
            );

            let handles: Vec<_> = (0..10)
                .map(|i| {
                    let repo = Arc::clone(&repo);
                    std::thread::spawn(move || {
                        repo.store(test_self_hosted_account(&format!("{i}.com")))
                            .expect("store failed")
                    })
                })
                .collect();

            handles
                .into_iter()
                .map(|h| h.join().expect("thread panicked"))
                .collect::<Vec<AccountId>>()
        };

        let repo = AccountRepository::new(path, transformer).expect("failed to create repo");
        for id in &ids {
            assert!(
                repo.get(*id).expect("get failed").is_some(),
                "account with id {id} missing after reload"
            );
        }
    }

    #[test]
    fn test_all_returns_empty_for_new_repo() {
        let (_dir, repo) = temp_repo();
        assert!(repo.all().expect("all failed").is_empty());
    }

    #[test]
    fn test_all_returns_all_accounts() {
        let (_dir, repo) = temp_repo();
        repo.store(test_wp_com_account("a.com"))
            .expect("store failed");
        repo.store(test_self_hosted_account("b.com"))
            .expect("store failed");

        let all = repo.all().expect("all failed");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_has_wp_com_account() {
        let (_dir, repo) = temp_repo();
        assert!(!repo.has_wp_com_account());

        repo.store(test_self_hosted_account("a.com"))
            .expect("store failed");
        assert!(!repo.has_wp_com_account());

        repo.store(test_wp_com_account("b.com"))
            .expect("store failed");
        assert!(repo.has_wp_com_account());
    }

    #[test]
    fn test_has_self_hosted_account() {
        let (_dir, repo) = temp_repo();
        assert!(!repo.has_self_hosted_account());

        repo.store(test_wp_com_account("a.com"))
            .expect("store failed");
        assert!(!repo.has_self_hosted_account());

        repo.store(test_self_hosted_account("b.com"))
            .expect("store failed");
        assert!(repo.has_self_hosted_account());
    }

    #[test]
    fn test_passwords_are_encrypted_on_disk() {
        let (_dir, repo) = temp_repo();
        repo.store(test_wp_com_account("example.com"))
            .expect("store failed");

        let raw = fs::read_to_string(&repo.file_path).expect("read failed");
        assert!(!raw.contains("secret"), "plaintext password found on disk");
    }

    #[test]
    fn test_password_round_trips_through_store_and_get() {
        let (_dir, repo) = temp_repo();
        let id = repo
            .store(test_wp_com_account("example.com"))
            .expect("store failed");

        let account = repo.get(id).expect("get failed").expect("account missing");
        match account {
            Account::WpCom { token, .. } => assert_eq!(token.0, "secret"),
            _ => panic!("expected WpCom variant"),
        }
    }

    #[test]
    fn test_all_returns_decrypted_passwords() {
        let (_dir, repo) = temp_repo();
        repo.store(test_wp_com_account("a.com"))
            .expect("store failed");
        repo.store(test_self_hosted_account("b.com"))
            .expect("store failed");

        let all = repo.all().expect("all failed");
        for account in &all {
            match account {
                Account::WpCom { token, .. } => {
                    assert_eq!(token.0, "secret");
                }
                Account::SelfHostedSite { password, .. } => {
                    assert_eq!(password.0, "secret");
                }
            }
        }
    }

    #[test]
    fn test_different_transformer_cannot_decrypt() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().to_string_lossy().to_string();

        let transformer1 = Arc::new(
            super::super::aes_gcm_transformer::AesGcmPasswordTransformer::new(
                "secret-one".to_string(),
            ),
        );
        let transformer2 = Arc::new(
            super::super::aes_gcm_transformer::AesGcmPasswordTransformer::new(
                "secret-two".to_string(),
            ),
        );

        let id = {
            let repo =
                AccountRepository::new(path.clone(), transformer1).expect("failed to create repo");
            repo.store(test_wp_com_account("example.com"))
                .expect("store failed")
        };

        let repo = AccountRepository::new(path, transformer2).expect("failed to create repo");
        assert!(repo.get(id).is_err());
    }

    #[test]
    fn test_corrupted_json_returns_error() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        fs::write(dir.path().join("accounts.json"), "not valid json {{{").unwrap();

        let result =
            AccountRepository::new(dir.path().to_string_lossy().to_string(), test_transformer());
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_file_returns_error() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        fs::write(dir.path().join("accounts.json"), "").unwrap();

        let result =
            AccountRepository::new(dir.path().to_string_lossy().to_string(), test_transformer());
        assert!(result.is_err());
    }

    #[test]
    fn test_uncreatable_path_returns_error() {
        let result =
            AccountRepository::new("/dev/null/impossible/path".to_string(), test_transformer());
        assert!(result.is_err());
    }
}
