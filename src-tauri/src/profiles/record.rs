use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorCode};

pub const PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProfile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub runtime_id: String,
    pub runtime_label: String,
    pub model_id: String,
    pub model_name: String,
    pub model_path: PathBuf,
    pub projector_path: Option<PathBuf>,
    pub host: String,
    pub port: u16,
    pub auto_select_port: bool,
    pub options: BTreeMap<String, ProfileOptionSetting>,
    pub environment: BTreeMap<String, String>,
    pub additional_arguments: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl LaunchProfile {
    pub fn input(&self) -> ProfileInput {
        ProfileInput {
            name: self.name.clone(),
            description: self.description.clone(),
            runtime_id: self.runtime_id.clone(),
            model_id: self.model_id.clone(),
            host: self.host.clone(),
            port: self.port,
            auto_select_port: self.auto_select_port,
            options: self.options.clone(),
            environment: self.environment.clone(),
            additional_arguments: self.additional_arguments.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub name: String,
    pub description: Option<String>,
    pub runtime_id: String,
    pub model_id: String,
    pub host: String,
    pub port: u16,
    pub auto_select_port: bool,
    #[serde(default)]
    pub options: BTreeMap<String, ProfileOptionSetting>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub additional_arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum ProfileOptionSetting {
    #[default]
    Default,
    Auto,
    Custom {
        value: String,
    },
}

pub fn normalize_input(mut input: ProfileInput) -> AppResult<ProfileInput> {
    input.name = input.name.trim().to_string();
    input.description = input
        .description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());
    input.runtime_id = input.runtime_id.trim().to_string();
    input.model_id = input.model_id.trim().to_string();
    input.host = input.host.trim().to_string();
    input
        .options
        .retain(|_, setting| *setting != ProfileOptionSetting::Default);

    if input.name.is_empty() || input.name.chars().count() > 120 {
        return Err(invalid_profile(
            "Profile name must contain between 1 and 120 characters.",
        ));
    }
    if input.runtime_id.is_empty() {
        return Err(invalid_profile("Choose a runtime for this profile."));
    }
    if input.model_id.is_empty() {
        return Err(invalid_profile("Choose a model for this profile."));
    }
    if input.host.is_empty() || input.host.contains('\0') {
        return Err(invalid_profile("Server host must not be empty."));
    }
    if input.port == 0 {
        return Err(invalid_profile("Server port must be between 1 and 65535."));
    }
    if input.options.len() > 512 {
        return Err(invalid_profile(
            "A profile contains too many option overrides.",
        ));
    }
    for (key, setting) in &input.options {
        if key.trim().is_empty() || key.contains('\0') {
            return Err(invalid_profile("A profile contains an invalid option key."));
        }
        if let ProfileOptionSetting::Custom { value } = setting {
            if value.contains('\0') {
                return Err(invalid_profile(
                    "An option value contains a null character.",
                ));
            }
        }
    }
    if input.environment.len() > 128 {
        return Err(invalid_profile(
            "A profile contains too many environment variables.",
        ));
    }
    let mut environment_keys = BTreeSet::new();
    for (key, value) in &input.environment {
        if !valid_environment_key(key) || value.contains('\0') {
            return Err(
                invalid_profile(format!("Environment variable {key:?} is not valid.")).with_hint(
                    "Use letters, digits, and underscores; the first character cannot be a digit.",
                ),
            );
        }
        if !environment_keys.insert(key.to_ascii_uppercase()) {
            return Err(invalid_profile(format!(
                "Environment variable {key:?} is duplicated with different letter casing."
            )));
        }
    }
    if input.additional_arguments.len() > 256 {
        return Err(invalid_profile(
            "A profile contains too many additional arguments.",
        ));
    }
    if input
        .additional_arguments
        .iter()
        .any(|argument| argument.contains('\0'))
    {
        return Err(invalid_profile(
            "An additional argument contains a null character.",
        ));
    }
    Ok(input)
}

fn valid_environment_key(key: &str) -> bool {
    let mut characters = key.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn invalid_profile(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::InvalidProfile, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ProfileInput {
        ProfileInput {
            name: "  Local CUDA  ".into(),
            description: Some("  everyday profile  ".into()),
            runtime_id: "runtime-1".into(),
            model_id: "model-1".into(),
            host: " 127.0.0.1 ".into(),
            port: 8080,
            auto_select_port: true,
            options: BTreeMap::from([("contextSize".into(), ProfileOptionSetting::Default)]),
            environment: BTreeMap::new(),
            additional_arguments: Vec::new(),
        }
    }

    #[test]
    fn normalizes_human_text_and_drops_default_options() {
        let normalized = normalize_input(input()).expect("valid profile");
        assert_eq!(normalized.name, "Local CUDA");
        assert_eq!(normalized.description.as_deref(), Some("everyday profile"));
        assert_eq!(normalized.host, "127.0.0.1");
        assert!(normalized.options.is_empty());
    }

    #[test]
    fn rejects_invalid_environment_names_and_ports() {
        let mut invalid = input();
        invalid.environment.insert("BAD=KEY".into(), "value".into());
        assert_eq!(
            normalize_input(invalid).expect_err("invalid key").code,
            ErrorCode::InvalidProfile
        );

        let mut invalid = input();
        invalid.port = 0;
        assert_eq!(
            normalize_input(invalid).expect_err("invalid port").code,
            ErrorCode::InvalidProfile
        );
    }
}
