use std::{collections::HashMap, path::PathBuf};

use color_eyre::{eyre::eyre, Result};
use email::{account::config::AccountConfig, config::Config};
use pimalaya_tui::config::toml::himalaya::AccountsConfig;
use serde::{Deserialize, Serialize};

use crate::account::config::TomlAccountConfig;

/// The structure representation of the user TOML configuration file.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TomlConfig {
    #[serde(alias = "name")]
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub accounts: HashMap<String, TomlAccountConfig>,
    pub account: Option<AccountsConfig>,

    pub repl: Option<ReplConfig>,
}

impl TomlConfig {
    pub fn repl_keybinds(&self) -> Option<&KeybindsStyle> {
        self.repl.as_ref().and_then(|c| c.keybinds())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum KeybindsStyle {
    #[default]
    Emacs,
    #[serde(alias = "vim")]
    Vi,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ReplConfig {
    pub keybinds: Option<KeybindsStyle>,
}

impl ReplConfig {
    pub fn keybinds(&self) -> Option<&KeybindsStyle> {
        self.keybinds.as_ref()
    }
}

impl From<TomlConfig> for Config {
    fn from(config: TomlConfig) -> Self {
        Self {
            display_name: config.display_name,
            signature: config.signature,
            signature_delim: config.signature_delim,
            downloads_dir: config.downloads_dir,
            accounts: config
                .accounts
                .into_iter()
                .map(|(name, config)| {
                    let mut config = AccountConfig::from(config);
                    config.name = name.clone();
                    (name, config)
                })
                .collect(),
        }
    }
}

impl pimalaya_tui::config::toml::TomlConfig for TomlConfig {
    type AccountConfig = TomlAccountConfig;

    fn project_name() -> &'static str {
        "himalaya"
    }

    fn get_default_account_config(&self) -> Result<(String, Self::AccountConfig)> {
        self.accounts
            .iter()
            .find_map(|(name, account)| {
                account
                    .default
                    .filter(|default| *default)
                    .map(|_| (name.to_owned(), account.clone()))
            })
            .ok_or_else(|| eyre!("cannot find default account"))
    }

    fn get_account_config(&self, name: &str) -> Result<(String, Self::AccountConfig)> {
        self.accounts
            .get(name)
            .map(|account| (name.to_owned(), account.clone()))
            .ok_or_else(|| eyre!("cannot find account {name}"))
    }
}
