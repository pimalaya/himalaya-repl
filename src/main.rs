pub mod account;
pub mod backend;
pub mod cli;
pub mod config;
pub mod envelope;
pub mod id_mapper;

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use backend::BackendKind;
use clap::Parser;
use cli::Cli;
use color_eyre::Result;
#[cfg(feature = "imap")]
use email::imap::ImapContextBuilder;
#[cfg(feature = "maildir")]
use email::maildir::MaildirContextBuilder;
#[cfg(feature = "notmuch")]
use email::notmuch::NotmuchContextBuilder;
#[cfg(feature = "sendmail")]
use email::sendmail::SendmailContextBuilder;
#[cfg(feature = "smtp")]
use email::smtp::SmtpContextBuilder;
use email::{backend::BackendBuilder, folder::list::ListFolders};

use pimalaya_tui::cli::tracing;
use reedline::{
    default_emacs_keybindings, ColumnarMenu, DefaultCompleter, DefaultPrompt, DefaultPromptSegment,
    Emacs, KeyCode, KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
};

use crate::{backend::ContextBuilder, config::Config};

#[tokio::main]
async fn main() -> Result<()> {
    tracing::install()?;

    let cli = Cli::parse();

    println!("Welcome to Himalaya REPL!");
    println!("Connecting…");

    let config = Config::from_paths_or_default(&cli.config_paths).await?;
    let (toml_account_config, account_config) = config.into_account_configs(None)?;
    let account_config = Arc::new(account_config);

    let backend = BackendBuilder::new(
        account_config.clone(),
        ContextBuilder {
            backend: toml_account_config.backend.unwrap_or(BackendKind::None),
            sending_backend: toml_account_config
                .message
                .and_then(|c| c.send)
                .and_then(|c| c.backend)
                .unwrap_or(BackendKind::None),

            #[cfg(feature = "imap")]
            imap: toml_account_config
                .imap
                .map(|imap| ImapContextBuilder::new(account_config.clone(), Arc::new(imap))),
            #[cfg(feature = "maildir")]
            maildir: toml_account_config.maildir.map(|maildir| {
                MaildirContextBuilder::new(account_config.clone(), Arc::new(maildir))
            }),
            #[cfg(feature = "notmuch")]
            notmuch: toml_account_config.notmuch.map(|notmuch| {
                NotmuchContextBuilder::new(account_config.clone(), Arc::new(notmuch))
            }),
            #[cfg(feature = "smtp")]
            smtp: toml_account_config
                .smtp
                .map(|smtp| SmtpContextBuilder::new(account_config.clone(), Arc::new(smtp))),
            #[cfg(feature = "sendmail")]
            sendmail: toml_account_config.sendmail.map(|sendmail| {
                SendmailContextBuilder::new(account_config.clone(), Arc::new(sendmail))
            }),
        },
    )
    .build()
    .await?;

    println!();

    let mut mode_kind = Mode::Unselected;
    let mut mode = UnselectedMode::new();

    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic(String::from("himalaya-repl")),
        DefaultPromptSegment::Empty,
    );

    let mut folder = Option::<&str>::None;

    loop {
        match mode.read_line(&prompt)? {
            Signal::Success(cmd) => match cmd.as_str() {
                "list" => {
                    let folders = backend.list_folders().await?;
                    println!("folders: {folders:?}");
                }
                "select" => {
                    // TODO: select folder
                    let folders = backend.list_folders().await?;
                    println!("folders: {folders:?}");
                }
                // "envelope list" => {
                //     let id_mapper = IdMapper::Dummy;
                //     let envelopes = backend
                //         .list_envelopes(
                //             "INBOX",
                //             ListEnvelopesOptions {
                //                 page_size: 10,
                //                 ..Default::default()
                //             },
                //         )
                //         .await?;
                //     let envelopes =
                //         Envelopes::try_from_lib(account_config.clone(), &id_mapper, envelopes)?;
                //     let table = EnvelopesTable::from(envelopes);
                //     println!("{table}");
                // }
                cmd => {
                    eprintln!("{cmd}: command not found");
                }
            },
            Signal::CtrlD | Signal::CtrlC => {
                println!("Bye!");
                break;
            }
        }
    }

    Ok(())
}

enum Mode {
    Unselected,
    Selected,
}

struct UnselectedMode(Reedline);

impl UnselectedMode {
    pub fn new() -> impl DerefMut<Target = Reedline> {
        let commands = vec!["create".into(), "list".into(), "select".into()];
        let completer = Box::new(DefaultCompleter::new_with_wordlen(commands.clone(), 2));
        let completion_menu = Box::new(ColumnarMenu::default().with_name("completion"));

        let mut keybinds = default_emacs_keybindings();
        keybinds.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("completion".to_string()),
                ReedlineEvent::MenuNext,
            ]),
        );

        let reedline = Reedline::create()
            .with_completer(completer)
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
            .with_edit_mode(Box::new(Emacs::new(keybinds)));

        Self(reedline)
    }
}

impl Deref for UnselectedMode {
    type Target = Reedline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UnselectedMode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
