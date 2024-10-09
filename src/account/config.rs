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
use email::{account::config::AccountConfig, template::config::TemplateConfig};
use pimalaya_tui::config::toml::himalaya::{
    BackendKind, EnvelopeConfig, FolderConfig, MessageConfig,
};
use serde::{Deserialize, Serialize};

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

    pub folder: Option<FolderConfig>,
    pub envelope: Option<EnvelopeConfig>,
    pub message: Option<MessageConfig>,
    pub template: Option<TemplateConfig>,

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

impl From<TomlAccountConfig> for AccountConfig {
    fn from(config: TomlAccountConfig) -> Self {
        Self {
            name: String::new(),
            email: config.email,
            display_name: config.display_name,
            signature: config.signature,
            signature_delim: config.signature_delim,
            downloads_dir: config.downloads_dir,

            #[cfg(feature = "pgp")]
            pgp: config.pgp,

            folder: config.folder.map(Into::into),
            envelope: config.envelope.map(Into::into),
            flag: None,
            message: config.message.map(Into::into),
            template: config.template,
        }
    }
}

impl TomlAccountConfig {
    pub fn envelope_list_table_preset(&self) -> Option<String> {
        self.envelope.as_ref().and_then(|c| c.list_table_preset())
    }

    pub fn envelope_list_table_unseen_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|c| c.list_table_unseen_char())
    }

    pub fn envelope_list_table_replied_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|c| c.list_table_replied_char())
    }

    pub fn envelope_list_table_flagged_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|c| c.list_table_flagged_char())
    }

    pub fn envelope_list_table_attachment_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|c| c.list_table_attachment_char())
    }

    pub fn envelope_list_table_id_color(&self) -> Option<Color> {
        self.envelope.as_ref().and_then(|c| c.list_table_id_color())
    }

    pub fn envelope_list_table_flags_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|c| c.list_table_flags_color())
    }

    pub fn envelope_list_table_subject_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|c| c.list_table_subject_color())
    }

    pub fn envelope_list_table_sender_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|c| c.list_table_sender_color())
    }

    pub fn envelope_list_table_date_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|c| c.list_table_date_color())
    }
}
