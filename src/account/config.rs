//! Deserialized account config module.
//!
//! This module contains the raw deserialized representation of an
//! account in the accounts section of the user configuration file.

use std::path::PathBuf;

use crossterm::style::Color;
#[cfg(feature = "pgp")]
use email::account::config::pgp::PgpConfig;
#[cfg(feature = "imap")]
use email::imap::config::ImapConfig;
#[cfg(feature = "maildir")]
use email::maildir::config::MaildirConfig;
#[cfg(feature = "notmuch")]
use email::notmuch::config::NotmuchConfig;
#[cfg(feature = "sendmail")]
use email::sendmail::config::SendmailConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::SmtpConfig;
use process::Command;
use serde::{Deserialize, Serialize};

use crate::backend::BackendKind;

// use crate::{
//     backend::BackendKind, envelope::config::EnvelopeConfig, flag::config::FlagConfig,
//     folder::config::FolderConfig, message::config::MessageConfig,
// };

/// Represents all existing kind of account config.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TomlAccountConfig {
    pub default: Option<bool>,
    pub email: String,
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub backend: Option<BackendKind>,

    #[cfg(feature = "pgp")]
    pub pgp: Option<PgpConfig>,

    // pub folder: Option<FolderConfig>,
    // pub envelope: Option<EnvelopeConfig>,
    // pub flag: Option<FlagConfig>,
    pub message: Option<MessageConfig>,
    // pub template: Option<TemplateConfig>,
    #[cfg(feature = "imap")]
    pub imap: Option<ImapConfig>,
    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirConfig>,
    #[cfg(feature = "notmuch")]
    pub notmuch: Option<NotmuchConfig>,
    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpConfig>,
    #[cfg(feature = "sendmail")]
    pub sendmail: Option<SendmailConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageConfig {
    pub send: Option<MessageSendConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageSendConfig {
    pub backend: Option<BackendKind>,
    pub save_copy: Option<bool>,
    pub pre_hook: Option<Command>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListAccountsTableConfig {
    pub preset: Option<String>,
    pub name_color: Option<Color>,
    pub backends_color: Option<Color>,
    pub default_color: Option<Color>,
}

// impl ListAccountsTableConfig {
//     pub fn preset(&self) -> &str {
//         self.preset.as_deref().unwrap_or(presets::ASCII_MARKDOWN)
//     }

//     pub fn name_color(&self) -> comfy_table::Color {
//         map_color(self.name_color.unwrap_or(Color::Green))
//     }

//     pub fn backends_color(&self) -> comfy_table::Color {
//         map_color(self.backends_color.unwrap_or(Color::Blue))
//     }

//     pub fn default_color(&self) -> comfy_table::Color {
//         map_color(self.default_color.unwrap_or(Color::Reset))
//     }
// }
