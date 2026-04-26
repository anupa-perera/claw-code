use std::collections::BTreeMap;
use std::fs::{self};
use std::io::{self};
use std::path::{Path, PathBuf};

use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::config::OAuthConfig;

/// Persisted OAuth access token bundle used by the CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub scopes: Vec<String>,
}

/// PKCE verifier/challenge pair generated for an OAuth authorization flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkceCodePair {
    pub verifier: String,
    pub challenge: String,
    pub challenge_method: PkceChallengeMethod,
}

/// Challenge algorithms supported by the local PKCE helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkceChallengeMethod {
    S256,
}

impl PkceChallengeMethod {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::S256 => "S256",
        }
    }
}

/// Parameters needed to build an authorization URL for browser-based login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthAuthorizationRequest {
    pub authorize_url: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: PkceChallengeMethod,
    pub extra_params: BTreeMap<String, String>,
}

/// Request body for exchanging an OAuth authorization code for tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthTokenExchangeRequest {
    pub grant_type: &'static str,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub code_verifier: String,
    pub state: String,
}

/// Request body for refreshing an existing OAuth token set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthRefreshRequest {
    pub grant_type: &'static str,
    pub refresh_token: String,
    pub client_id: String,
    pub scopes: Vec<String>,
}

/// Parsed query parameters returned to the local OAuth callback endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredOAuthCredentials {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_at: Option<u64>,
    #[serde(default)]
    scopes: Vec<String>,
}

impl From<OAuthTokenSet> for StoredOAuthCredentials {
    fn from(value: OAuthTokenSet) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            expires_at: value.expires_at,
            scopes: value.scopes,
        }
    }
}

impl From<StoredOAuthCredentials> for OAuthTokenSet {
    fn from(value: StoredOAuthCredentials) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            expires_at: value.expires_at,
            scopes: value.scopes,
        }
    }
}

impl OAuthAuthorizationRequest {
    #[must_use]
    pub fn from_config(
        config: &OAuthConfig,
        redirect_uri: impl Into<String>,
        state: impl Into<String>,
        pkce: &PkceCodePair,
    ) -> Self {
        Self {
            authorize_url: config.authorize_url.clone(),
            client_id: config.client_id.clone(),
            redirect_uri: redirect_uri.into(),
            scopes: config.scopes.clone(),
            state: state.into(),
            code_challenge: pkce.challenge.clone(),
            code_challenge_method: pkce.challenge_method,
            extra_params: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_extra_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_params.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn build_url(&self) -> String {
        let mut params = vec![
            ("response_type", "code".to_string()),
            ("client_id", self.client_id.clone()),
            ("redirect_uri", self.redirect_uri.clone()),
            ("scope", self.scopes.join(" ")),
            ("state", self.state.clone()),
            ("code_challenge", self.code_challenge.clone()),
            (
                "code_challenge_method",
                self.code_challenge_method.as_str().to_string(),
            ),
        ];
        params.extend(
            self.extra_params
                .iter()
                .map(|(key, value)| (key.as_str(), value.clone())),
        );
        let query = params
            .into_iter()
            .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(&value)))
            .collect::<Vec<_>>()
            .join("&");
        format!(
            "{}{}{}",
            self.authorize_url,
            if self.authorize_url.contains('?') {
                '&'
            } else {
                '?'
            },
            query
        )
    }
}

impl OAuthTokenExchangeRequest {
    #[must_use]
    pub fn from_config(
        config: &OAuthConfig,
        code: impl Into<String>,
        state: impl Into<String>,
        verifier: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            grant_type: "authorization_code",
            code: code.into(),
            redirect_uri: redirect_uri.into(),
            client_id: config.client_id.clone(),
            code_verifier: verifier.into(),
            state: state.into(),
        }
    }

    #[must_use]
    pub fn form_params(&self) -> BTreeMap<&str, String> {
        BTreeMap::from([
            ("grant_type", self.grant_type.to_string()),
            ("code", self.code.clone()),
            ("redirect_uri", self.redirect_uri.clone()),
            ("client_id", self.client_id.clone()),
            ("code_verifier", self.code_verifier.clone()),
            ("state", self.state.clone()),
        ])
    }
}

impl OAuthRefreshRequest {
    #[must_use]
    pub fn from_config(
        config: &OAuthConfig,
        refresh_token: impl Into<String>,
        scopes: Option<Vec<String>>,
    ) -> Self {
        Self {
            grant_type: "refresh_token",
            refresh_token: refresh_token.into(),
            client_id: config.client_id.clone(),
            scopes: scopes.unwrap_or_else(|| config.scopes.clone()),
        }
    }

    #[must_use]
    pub fn form_params(&self) -> BTreeMap<&str, String> {
        BTreeMap::from([
            ("grant_type", self.grant_type.to_string()),
            ("refresh_token", self.refresh_token.clone()),
            ("client_id", self.client_id.clone()),
            ("scope", self.scopes.join(" ")),
        ])
    }
}

pub fn generate_pkce_pair() -> io::Result<PkceCodePair> {
    let verifier = generate_random_token(32)?;
    Ok(PkceCodePair {
        challenge: code_challenge_s256(&verifier),
        verifier,
        challenge_method: PkceChallengeMethod::S256,
    })
}

pub fn generate_state() -> io::Result<String> {
    generate_random_token(32)
}

#[must_use]
pub fn code_challenge_s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64url_encode(&digest)
}

#[must_use]
pub fn loopback_redirect_uri(port: u16) -> String {
    format!("http://localhost:{port}/callback")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedApiKeyProvider {
    Anthropic,
    OpenAi,
    OpenRouter,
    Xai,
}

impl SavedApiKeyProvider {
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::OpenRouter => "openrouter",
            Self::Xai => "xai",
        }
    }
}

pub fn credentials_path() -> io::Result<PathBuf> {
    Ok(credentials_home_dir()?.join("credentials.json"))
}

pub fn load_saved_api_key(provider: SavedApiKeyProvider) -> io::Result<Option<String>> {
    let path = provider_auth_path()?;
    let root = read_json_root(&path)?;
    let Some(providers) = root.get("providers").and_then(Value::as_object) else {
        return Ok(None);
    };
    let Some(provider_root) = providers.get(provider.key()).and_then(Value::as_object) else {
        return Ok(None);
    };
    Ok(provider_root
        .get("apiKey")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned))
}

pub fn save_api_key(provider: SavedApiKeyProvider, api_key: &str) -> io::Result<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "api key must not be empty",
        ));
    }

    let path = provider_auth_path()?;
    let mut root = read_json_root(&path)?;
    root.insert("version".to_string(), Value::from(1));

    let providers = root
        .entry("providers".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "provider-auth.json field 'providers' must be a JSON object",
            )
        })?;

    let provider_root = providers
        .entry(provider.key().to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "provider-auth.json provider entry must be a JSON object",
            )
        })?;

    provider_root.insert("apiKey".to_string(), Value::String(trimmed.to_string()));
    write_json_root(&path, &root)
}

pub fn clear_saved_api_key(provider: SavedApiKeyProvider) -> io::Result<()> {
    let path = provider_auth_path()?;
    let mut root = read_json_root(&path)?;
    let Some(providers) = root.get_mut("providers").and_then(Value::as_object_mut) else {
        return Ok(());
    };

    let mut remove_provider = false;
    if let Some(provider_root) = providers
        .get_mut(provider.key())
        .and_then(Value::as_object_mut)
    {
        provider_root.remove("apiKey");
        remove_provider = provider_root.is_empty();
    }
    if remove_provider {
        providers.remove(provider.key());
    }
    if providers.is_empty() {
        root.remove("providers");
    }
    if !root.contains_key("providers") && root.keys().all(|key| key == "version") {
        root.remove("version");
    }

    write_json_root_or_remove(&path, &root)
}

pub fn load_oauth_credentials() -> io::Result<Option<OAuthTokenSet>> {
    let path = credentials_path()?;
    let root = read_json_root(&path)?;
    let Some(oauth) = root.get("oauth") else {
        return Ok(None);
    };
    if oauth.is_null() {
        return Ok(None);
    }
    let stored = serde_json::from_value::<StoredOAuthCredentials>(oauth.clone())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some(stored.into()))
}

pub fn save_oauth_credentials(token_set: &OAuthTokenSet) -> io::Result<()> {
    let path = credentials_path()?;
    let mut root = read_json_root(&path)?;
    root.insert(
        "oauth".to_string(),
        serde_json::to_value(StoredOAuthCredentials::from(token_set.clone()))
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
    );
    write_json_root(&path, &root)
}

pub fn clear_oauth_credentials() -> io::Result<()> {
    let path = credentials_path()?;
    let mut root = read_json_root(&path)?;
    root.remove("oauth");
    write_json_root_or_remove(&path, &root)
}

pub fn parse_oauth_callback_request_target(target: &str) -> Result<OAuthCallbackParams, String> {
    let (path, query) = target
        .split_once('?')
        .map_or((target, ""), |(path, query)| (path, query));
    if path != "/callback" {
        return Err(format!("unexpected callback path: {path}"));
    }
    parse_oauth_callback_query(query)
}

pub fn parse_oauth_callback_query(query: &str) -> Result<OAuthCallbackParams, String> {
    let mut params = BTreeMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair
            .split_once('=')
            .map_or((pair, ""), |(key, value)| (key, value));
        params.insert(percent_decode(key)?, percent_decode(value)?);
    }
    Ok(OAuthCallbackParams {
        code: params.get("code").cloned(),
        state: params.get("state").cloned(),
        error: params.get("error").cloned(),
        error_description: params.get("error_description").cloned(),
    })
}

fn generate_random_token(bytes: usize) -> io::Result<String> {
    let mut buffer = vec![0_u8; bytes];
    getrandom(&mut buffer)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
    Ok(base64url_encode(&buffer))
}

fn credentials_home_dir() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("CLAW_CONFIG_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = crate::config::resolve_user_home_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not determine the user home directory from HOME, USERPROFILE, or HOMEDRIVE/HOMEPATH",
        )
    })?;
    Ok(home.join(".claw"))
}

fn provider_auth_path() -> io::Result<PathBuf> {
    Ok(credentials_home_dir()?.join("provider-auth.json"))
}

fn read_json_root(path: &Path) -> io::Result<Map<String, Value>> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            if contents.trim().is_empty() {
                return Ok(Map::new());
            }
            serde_json::from_str::<Value>(&contents)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
                .as_object()
                .cloned()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "credentials file must contain a JSON object",
                    )
                })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Map::new()),
        Err(error) => Err(error),
    }
}

fn write_json_root_or_remove(path: &Path, root: &Map<String, Value>) -> io::Result<()> {
    if root.is_empty() {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    } else {
        write_json_root(path, root)
    }
}

fn write_json_root(path: &Path, root: &Map<String, Value>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let rendered = serde_json::to_string_pretty(&Value::Object(root.clone()))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, format!("{rendered}\n"))?;
    fs::rename(temp_path, path)
}

fn base64url_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::new();
    let mut index = 0;
    while index + 3 <= bytes.len() {
        let block = (u32::from(bytes[index]) << 16)
            | (u32::from(bytes[index + 1]) << 8)
            | u32::from(bytes[index + 2]);
        output.push(TABLE[((block >> 18) & 0x3F) as usize] as char);
        output.push(TABLE[((block >> 12) & 0x3F) as usize] as char);
        output.push(TABLE[((block >> 6) & 0x3F) as usize] as char);
        output.push(TABLE[(block & 0x3F) as usize] as char);
        index += 3;
    }
    match bytes.len().saturating_sub(index) {
        1 => {
            let block = u32::from(bytes[index]) << 16;
            output.push(TABLE[((block >> 18) & 0x3F) as usize] as char);
            output.push(TABLE[((block >> 12) & 0x3F) as usize] as char);
        }
        2 => {
            let block = (u32::from(bytes[index]) << 16) | (u32::from(bytes[index + 1]) << 8);
            output.push(TABLE[((block >> 18) & 0x3F) as usize] as char);
            output.push(TABLE[((block >> 12) & 0x3F) as usize] as char);
            output.push(TABLE[((block >> 6) & 0x3F) as usize] as char);
        }
        _ => {}
    }
    output
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => {
                use std::fmt::Write as _;
                let _ = write!(&mut encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

fn percent_decode(value: &str) -> Result<String, String> {
    let mut decoded = Vec::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hi = decode_hex(bytes[index + 1])?;
                let lo = decode_hex(bytes[index + 2])?;
                decoded.push((hi << 4) | lo);
                index += 3;
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).map_err(|error| error.to_string())
}

fn decode_hex(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid percent byte: {byte}")),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        clear_oauth_credentials, clear_saved_api_key, code_challenge_s256, credentials_path,
        generate_pkce_pair, generate_state, load_oauth_credentials, load_saved_api_key,
        loopback_redirect_uri, parse_oauth_callback_query, parse_oauth_callback_request_target,
        save_api_key, save_oauth_credentials, OAuthAuthorizationRequest, OAuthConfig,
        OAuthRefreshRequest, OAuthTokenExchangeRequest, OAuthTokenSet, SavedApiKeyProvider,
    };

    fn sample_config() -> OAuthConfig {
        OAuthConfig {
            client_id: "runtime-client".to_string(),
            authorize_url: "https://console.test/oauth/authorize".to_string(),
            token_url: "https://console.test/oauth/token".to_string(),
            callback_port: Some(4545),
            manual_redirect_url: Some("https://console.test/oauth/callback".to_string()),
            scopes: vec!["org:read".to_string(), "user:write".to_string()],
        }
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        crate::test_env_lock()
    }

    fn temp_config_home() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "runtime-oauth-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    fn restore_env_var(key: &str, original: Option<OsString>) {
        match original {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn s256_challenge_matches_expected_vector() {
        assert_eq!(
            code_challenge_s256("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn generates_pkce_pair_and_state() {
        let pair = generate_pkce_pair().expect("pkce pair");
        let state = generate_state().expect("state");
        assert!(!pair.verifier.is_empty());
        assert!(!pair.challenge.is_empty());
        assert!(!state.is_empty());
    }

    #[test]
    fn builds_authorize_url_and_form_requests() {
        let config = sample_config();
        let pair = generate_pkce_pair().expect("pkce");
        let url = OAuthAuthorizationRequest::from_config(
            &config,
            loopback_redirect_uri(4545),
            "state-123",
            &pair,
        )
        .with_extra_param("login_hint", "user@example.com")
        .build_url();
        assert!(url.starts_with("https://console.test/oauth/authorize?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=runtime-client"));
        assert!(url.contains("scope=org%3Aread%20user%3Awrite"));
        assert!(url.contains("login_hint=user%40example.com"));

        let exchange = OAuthTokenExchangeRequest::from_config(
            &config,
            "auth-code",
            "state-123",
            pair.verifier,
            loopback_redirect_uri(4545),
        );
        assert_eq!(
            exchange.form_params().get("grant_type").map(String::as_str),
            Some("authorization_code")
        );

        let refresh = OAuthRefreshRequest::from_config(&config, "refresh-token", None);
        assert_eq!(
            refresh.form_params().get("scope").map(String::as_str),
            Some("org:read user:write")
        );
    }

    #[test]
    fn oauth_credentials_round_trip_and_clear_preserves_other_fields() {
        let _guard = env_lock();
        let config_home = temp_config_home();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        let path = credentials_path().expect("credentials path");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        std::fs::write(&path, "{\"other\":\"value\"}\n").expect("seed credentials");

        let token_set = OAuthTokenSet {
            access_token: "access-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(123),
            scopes: vec!["scope:a".to_string()],
        };
        save_oauth_credentials(&token_set).expect("save credentials");
        assert_eq!(
            load_oauth_credentials().expect("load credentials"),
            Some(token_set)
        );
        let saved = std::fs::read_to_string(&path).expect("read saved file");
        assert!(saved.contains("\"other\": \"value\""));
        assert!(saved.contains("\"oauth\""));

        clear_oauth_credentials().expect("clear credentials");
        assert_eq!(load_oauth_credentials().expect("load cleared"), None);
        let cleared = std::fs::read_to_string(&path).expect("read cleared file");
        assert!(cleared.contains("\"other\": \"value\""));
        assert!(!cleared.contains("\"oauth\""));

        std::env::remove_var("CLAW_CONFIG_HOME");
        std::fs::remove_dir_all(config_home).expect("cleanup temp dir");
    }

    #[test]
    fn provider_api_keys_round_trip_and_preserve_other_entries() {
        let _guard = env_lock();
        let config_home = temp_config_home();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        let path = config_home.join("provider-auth.json");
        std::fs::create_dir_all(&config_home).expect("create config home");
        std::fs::write(
            &path,
            "{\n  \"version\": 1,\n  \"providers\": {\n    \"openrouter\": { \"apiKey\": \"or-test\" },\n    \"xai\": { \"note\": \"keep\" }\n  }\n}\n",
        )
        .expect("seed provider auth");

        save_api_key(SavedApiKeyProvider::Anthropic, "anthropic-test").expect("save anthropic key");
        assert_eq!(
            load_saved_api_key(SavedApiKeyProvider::Anthropic).expect("load anthropic key"),
            Some("anthropic-test".to_string())
        );
        assert_eq!(
            load_saved_api_key(SavedApiKeyProvider::OpenRouter).expect("load openrouter key"),
            Some("or-test".to_string())
        );

        clear_saved_api_key(SavedApiKeyProvider::Anthropic).expect("clear anthropic key");
        assert_eq!(
            load_saved_api_key(SavedApiKeyProvider::Anthropic).expect("load cleared anthropic key"),
            None
        );

        let saved = std::fs::read_to_string(&path).expect("read saved provider auth");
        assert!(saved.contains("\"openrouter\""));
        assert!(saved.contains("\"or-test\""));
        assert!(saved.contains("\"note\": \"keep\""));

        std::env::remove_var("CLAW_CONFIG_HOME");
        std::fs::remove_dir_all(config_home).expect("cleanup temp dir");
    }

    #[test]
    fn clearing_last_saved_api_key_removes_provider_auth_file() {
        let _guard = env_lock();
        let config_home = temp_config_home();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        let path = config_home.join("provider-auth.json");
        std::fs::create_dir_all(&config_home).expect("create config home");

        save_api_key(SavedApiKeyProvider::OpenRouter, "or-test").expect("save openrouter key");
        assert!(path.exists());

        clear_saved_api_key(SavedApiKeyProvider::OpenRouter).expect("clear openrouter key");
        assert!(
            !path.exists(),
            "provider-auth.json should be removed when empty"
        );

        std::env::remove_var("CLAW_CONFIG_HOME");
        std::fs::remove_dir_all(config_home).expect("cleanup temp dir");
    }

    #[test]
    fn clearing_last_oauth_credentials_removes_credentials_file() {
        let _guard = env_lock();
        let config_home = temp_config_home();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        let path = credentials_path().expect("credentials path");

        save_oauth_credentials(&OAuthTokenSet {
            access_token: "access-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(123),
            scopes: vec!["scope:a".to_string()],
        })
        .expect("save credentials");
        assert!(path.exists());

        clear_oauth_credentials().expect("clear credentials");
        assert!(
            !path.exists(),
            "credentials.json should be removed when empty"
        );

        std::env::remove_var("CLAW_CONFIG_HOME");
        std::fs::remove_dir_all(config_home).expect("cleanup temp dir");
    }

    #[test]
    fn credentials_home_dir_uses_userprofile_when_home_is_missing() {
        let _guard = env_lock();
        let original_claw_config_home = std::env::var_os("CLAW_CONFIG_HOME");
        let original_home = std::env::var_os("HOME");
        let original_userprofile = std::env::var_os("USERPROFILE");
        let original_homedrive = std::env::var_os("HOMEDRIVE");
        let original_homepath = std::env::var_os("HOMEPATH");

        std::env::remove_var("CLAW_CONFIG_HOME");
        std::env::remove_var("HOME");
        std::env::remove_var("HOMEDRIVE");
        std::env::remove_var("HOMEPATH");
        std::env::set_var("USERPROFILE", r"C:\Users\TestUser");

        let path = credentials_path().expect("credentials path should resolve");
        assert_eq!(
            path,
            std::path::PathBuf::from(r"C:\Users\TestUser\.claw\credentials.json")
        );

        restore_env_var("CLAW_CONFIG_HOME", original_claw_config_home);
        restore_env_var("HOME", original_home);
        restore_env_var("USERPROFILE", original_userprofile);
        restore_env_var("HOMEDRIVE", original_homedrive);
        restore_env_var("HOMEPATH", original_homepath);
    }

    #[test]
    fn parses_callback_query_and_target() {
        let params =
            parse_oauth_callback_query("code=abc123&state=state-1&error_description=needs%20login")
                .expect("parse query");
        assert_eq!(params.code.as_deref(), Some("abc123"));
        assert_eq!(params.state.as_deref(), Some("state-1"));
        assert_eq!(params.error_description.as_deref(), Some("needs login"));

        let params = parse_oauth_callback_request_target("/callback?code=abc&state=xyz")
            .expect("parse callback target");
        assert_eq!(params.code.as_deref(), Some("abc"));
        assert_eq!(params.state.as_deref(), Some("xyz"));
        assert!(parse_oauth_callback_request_target("/wrong?code=abc").is_err());
    }
}
