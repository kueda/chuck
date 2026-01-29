use crate::auth::{AuthError, AuthToken, TokenStorage, KeyringStorage};
use std::sync::{Arc, Mutex, OnceLock};

/// Thread-safe authentication cache that lazily initializes keyring storage
/// and caches the token in memory to minimize keychain access prompts
pub struct AuthCache {
    storage: OnceLock<Arc<KeyringStorage>>,
    storage_init_attempted: Mutex<bool>,
    cached_token: Mutex<Option<Option<AuthToken>>>,
}

impl AuthCache {
    pub fn new() -> Self {
        Self {
            storage: OnceLock::new(),
            storage_init_attempted: Mutex::new(false),
            cached_token: Mutex::new(None),
        }
    }

    /// Get or create the KeyringStorage (triggers keychain access on first call)
    fn get_or_create_storage(&self) -> Result<&KeyringStorage, AuthError> {
        // Try to get existing storage
        if let Some(storage) = self.storage.get() {
            return Ok(storage.as_ref());
        }

        // Check if we already tried and failed
        {
            let attempted = self.storage_init_attempted.lock().unwrap();
            if *attempted {
                return Err(AuthError::OAuthFailed(
                    "Keyring storage initialization failed previously".to_string()
                ));
            }
        }

        // Mark that we're attempting initialization
        *self.storage_init_attempted.lock().unwrap() = true;

        // Try to initialize
        let storage = KeyringStorage::new()?;
        let arc_storage = Arc::new(storage);

        // Try to set it (might fail if another thread beat us to it, which is fine)
        let _ = self.storage.set(arc_storage);

        // Get the value (either ours or the other thread's)
        self.storage
            .get()
            .map(|s| s.as_ref())
            .ok_or_else(|| AuthError::OAuthFailed("Failed to initialize storage".to_string()))
    }

    /// Load token - checks cache first, only accesses keychain if not cached
    pub fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
        // Check if we've already loaded (or attempted to load) the token
        {
            let cached = self.cached_token.lock().unwrap();
            if let Some(ref token_option) = *cached {
                // We've cached the result (either Some(token) or None)
                return Ok(token_option.clone());
            }
        }

        // Not cached yet, load from keychain
        let storage = self.get_or_create_storage()?;
        let token = storage.load_token()?;

        // Cache the result
        *self.cached_token.lock().unwrap() = Some(token.clone());

        Ok(token)
    }

    /// Save token - updates both keychain and cache
    pub fn save_token(&self, token: &AuthToken) -> Result<(), AuthError> {
        let storage = self.get_or_create_storage()?;
        storage.save_token(token)?;

        // Update cache
        *self.cached_token.lock().unwrap() = Some(Some(token.clone()));

        Ok(())
    }

    /// Clear token - removes from both keychain and cache
    pub fn clear_token(&self) -> Result<(), AuthError> {
        let storage = self.get_or_create_storage()?;
        storage.clear_token()?;

        // Clear cache
        *self.cached_token.lock().unwrap() = Some(None);

        Ok(())
    }
}

impl Default for AuthCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenStorage for AuthCache {
    fn save_token(&self, token: &AuthToken) -> Result<(), AuthError> {
        self.save_token(token)
    }

    fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
        self.load_token()
    }

    fn clear_token(&self) -> Result<(), AuthError> {
        self.clear_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyring::mock::default_credential_builder;
    use keyring::Entry;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Counter to track KeyringStorage::new() calls
    static STORAGE_NEW_CALLS: AtomicUsize = AtomicUsize::new(0);

    // Mock version of KeyringStorage for testing
    pub struct MockKeyringStorage {
        entry: keyring::Entry,
    }

    impl MockKeyringStorage {
        pub fn new() -> Result<Self, AuthError> {
            // Increment counter
            STORAGE_NEW_CALLS.fetch_add(1, Ordering::SeqCst);

            // Use mock credential store - Entry::new will use the mock builder we set
            let entry = Entry::new("Chuck", "iNaturalist access token")
                .map_err(|e| AuthError::OAuthFailed(format!("Mock keyring unavailable: {e}")))?;

            Ok(Self { entry })
        }

        fn get_entry(&self) -> &keyring::Entry {
            &self.entry
        }
    }

    impl TokenStorage for MockKeyringStorage {
        fn save_token(&self, token: &AuthToken) -> Result<(), AuthError> {
            let token_json = serde_json::to_string(token)
                .map_err(AuthError::JsonError)?;

            self.get_entry().set_password(&token_json)
                .map_err(|e| AuthError::OAuthFailed(format!("Failed to save to mock keyring: {e}")))
        }

        fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
            match self.get_entry().get_password() {
                Ok(token_json) => {
                    let token: AuthToken = serde_json::from_str(&token_json)
                        .map_err(AuthError::JsonError)?;

                    if token.is_expired() {
                        return Err(AuthError::TokenExpired);
                    }

                    Ok(Some(token))
                }
                Err(_) => Ok(None),
            }
        }

        fn clear_token(&self) -> Result<(), AuthError> {
            match self.get_entry().delete_credential() {
                Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(AuthError::OAuthFailed(format!("Failed to clear mock keyring: {e}"))),
            }
        }
    }

    // Mock AuthCache that uses MockKeyringStorage
    pub struct MockAuthCache {
        storage: OnceLock<Arc<MockKeyringStorage>>,
        storage_init_attempted: Mutex<bool>,
        cached_token: Mutex<Option<Option<AuthToken>>>,
    }

    impl MockAuthCache {
        pub fn new() -> Self {
            Self {
                storage: OnceLock::new(),
                storage_init_attempted: Mutex::new(false),
                cached_token: Mutex::new(None),
            }
        }

        fn get_or_create_storage(&self) -> Result<&MockKeyringStorage, AuthError> {
            if let Some(storage) = self.storage.get() {
                return Ok(storage.as_ref());
            }

            {
                let attempted = self.storage_init_attempted.lock().unwrap();
                if *attempted {
                    return Err(AuthError::OAuthFailed(
                        "Mock storage initialization failed previously".to_string()
                    ));
                }
            }

            *self.storage_init_attempted.lock().unwrap() = true;

            let storage = MockKeyringStorage::new()?;
            let arc_storage = Arc::new(storage);

            let _ = self.storage.set(arc_storage);

            self.storage
                .get()
                .map(|s| s.as_ref())
                .ok_or_else(|| AuthError::OAuthFailed("Failed to initialize mock storage".to_string()))
        }

        pub fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
            {
                let cached = self.cached_token.lock().unwrap();
                if let Some(ref token_option) = *cached {
                    return Ok(token_option.clone());
                }
            }

            let storage = self.get_or_create_storage()?;
            let token = storage.load_token()?;

            *self.cached_token.lock().unwrap() = Some(token.clone());

            Ok(token)
        }
    }

    fn reset_storage_call_count() {
        STORAGE_NEW_CALLS.store(0, Ordering::SeqCst);
    }

    fn get_storage_call_count() -> usize {
        STORAGE_NEW_CALLS.load(Ordering::SeqCst)
    }

    #[test]
    #[serial_test::serial]
    fn test_new_cache_has_no_storage() {
        let cache = MockAuthCache::new();
        assert!(cache.storage.get().is_none());
    }

    #[test]
    #[serial_test::serial]
    fn test_multiple_load_token_calls_only_create_storage_once() {
        // Use mock credential store for this test
        keyring::set_default_credential_builder(default_credential_builder());

        reset_storage_call_count();
        let cache = MockAuthCache::new();

        let result1 = cache.load_token();
        let result2 = cache.load_token();
        let result3 = cache.load_token();

        // All three calls should return the same result
        match (&result1, &result2, &result3) {
            (Ok(token1), Ok(token2), Ok(token3)) => {
                assert_eq!(token1.is_some(), token2.is_some());
                assert_eq!(token2.is_some(), token3.is_some());
            },
            (Err(_), Err(_), Err(_)) => {
                // All failed consistently - that's fine
            },
            _ => panic!("Inconsistent results from load_token calls"),
        }

        // CRITICAL ASSERTION: Storage should only be created ONCE
        let call_count = get_storage_call_count();
        assert_eq!(
            call_count, 1,
            "MockKeyringStorage::new() should be called exactly once, but was called {call_count} times"
        );

        // Verify token was cached
        assert!(
            cache.cached_token.lock().unwrap().is_some(),
            "Token should be cached after first load_token call"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_load_token_caches_result() {
        keyring::set_default_credential_builder(default_credential_builder());

        let cache = MockAuthCache::new();

        // Call load_token once
        let _ = cache.load_token();

        // Verify the token is cached
        let cached = cache.cached_token.lock().unwrap();
        assert!(
            cached.is_some(),
            "Token result should be cached after first load_token call"
        );
    }
}
