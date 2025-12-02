pub mod error;
pub mod jwt;
pub mod oauth;
pub mod token;
mod token_storage;
mod file_storage;
mod custom_file_storage;
mod storage_config;
mod storage_factory;
#[cfg(feature = "keyring-storage")]
mod keyring_storage;

pub use error::AuthError;
pub use oauth::{authenticate_user};
pub use jwt::{fetch_jwt};
pub use token::{load_auth_token, save_auth_token, clear_auth_token, AuthToken};
pub use token_storage::TokenStorage;
pub use file_storage::FileStorage;
pub use custom_file_storage::CustomFileStorage;
pub use storage_config::{StorageBackendConfig, StorageBackendType};
pub use storage_factory::{StorageFactory, StorageInstance};
#[cfg(feature = "keyring-storage")]
pub use keyring_storage::KeyringStorage;
