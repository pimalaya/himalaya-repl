pub mod account;
pub mod backend;
pub mod cli;
pub mod config;
pub mod editor;
pub mod envelope;
pub mod id_mapper;

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use clap::Parser;
use cli::Cli;
use color_eyre::{eyre::eyre, Result};
use crossterm::style::Color;
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
use email::{
    backend::BackendBuilder,
    envelope::{
        list::{ListEnvelopes, ListEnvelopesOptions},
        Id,
    },
    folder::list::ListFolders,
    message::{
        copy::CopyMessages, delete::DeleteMessages, get::GetMessages, r#move::MoveMessages, Message,
    },
};
use pimalaya_tui::{
    cli::tracing,
    config::toml::{himalaya::BackendKind, TomlConfig as _},
    prompt,
};
use reedline::{
    default_emacs_keybindings, default_vi_insert_keybindings, default_vi_normal_keybindings,
    ColumnarMenu, DefaultCompleter, DefaultPrompt, DefaultPromptSegment, Emacs, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal, Vi,
};

use crate::{
    backend::ContextBuilder,
    config::{KeybindsStyle, TomlConfig},
    envelope::{Envelopes, EnvelopesTable},
    id_mapper::IdMapper,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing::install()?;

    let cli = Cli::parse();

    println!("Welcome to Himalaya REPL!");
    println!("Setting up backends…");

    let toml_cfg = TomlConfig::from_paths_or_default(&cli.config_paths).await?;
    let keybinds = toml_cfg.repl_keybinds().cloned().unwrap_or_default();
    let (toml_account_cfg, account_cfg) = toml_cfg.into_account_configs(None)?;

    let account_cfg = Arc::new(account_cfg);

    let backend =
        BackendBuilder::new(
            account_cfg.clone(),
            ContextBuilder {
                backend: toml_account_cfg
                    .backend
                    .clone()
                    .unwrap_or(BackendKind::None),
                sending_backend: toml_account_cfg
                    .message
                    .as_ref()
                    .and_then(|c| c.send.as_ref())
                    .and_then(|c| c.backend.clone())
                    .unwrap_or(BackendKind::None),

                #[cfg(feature = "imap")]
                imap: toml_account_cfg.imap.as_ref().map(|imap| {
                    ImapContextBuilder::new(account_cfg.clone(), Arc::new(imap.clone()))
                }),
                #[cfg(feature = "maildir")]
                maildir: toml_account_cfg.maildir.as_ref().map(|maildir| {
                    MaildirContextBuilder::new(account_cfg.clone(), Arc::new(maildir.clone()))
                }),
                #[cfg(feature = "notmuch")]
                notmuch: toml_account_cfg.notmuch.as_ref().map(|notmuch| {
                    NotmuchContextBuilder::new(account_cfg.clone(), Arc::new(notmuch.clone()))
                }),
                #[cfg(feature = "smtp")]
                smtp: toml_account_cfg.smtp.as_ref().map(|smtp| {
                    SmtpContextBuilder::new(account_cfg.clone(), Arc::new(smtp.clone()))
                }),
                #[cfg(feature = "sendmail")]
                sendmail: toml_account_cfg.sendmail.as_ref().map(|sendmail| {
                    SendmailContextBuilder::new(account_cfg.clone(), Arc::new(sendmail.clone()))
                }),
            },
        )
        .build()
        .await?;

    println!();

    let mut mode = UnselectedMode::new(keybinds);

    let mut folder = Option::<String>::None;

    loop {
        let prompt = match folder.as_ref() {
            Some(folder) => DefaultPrompt::new(
                DefaultPromptSegment::Basic(String::from("himalaya-repl")),
                DefaultPromptSegment::Basic(format!("[{folder}]")),
            ),
            None => DefaultPrompt::new(
                DefaultPromptSegment::Basic(String::from("himalaya-repl")),
                DefaultPromptSegment::Empty,
            ),
        };

        match mode.read_line(&prompt)? {
            Signal::Success(cmd) => match cmd.trim() {
                "select" => {
                    let folders = backend.list_folders().await?.into_iter().map(|f| f.name);
                    let f = prompt::item("Select a folder:", folders, None)?;
                    folder = Some(f);
                }
                "unselect" => {
                    folder = None;
                }
                "list" => {
                    let Some(folder) = folder.as_deref() else {
                        eprintln!("Please select a folder first");
                        continue;
                    };

                    let id_mapper = IdMapper::Dummy;
                    let envelopes = backend
                        .list_envelopes(
                            folder,
                            ListEnvelopesOptions {
                                page_size: 10,
                                ..Default::default()
                            },
                        )
                        .await?;
                    let envelopes =
                        Envelopes::try_from_lib(account_cfg.clone(), &id_mapper, envelopes)?;
                    let table = EnvelopesTable::from(envelopes)
                        .with_some_preset(toml_account_cfg.envelope_list_table_preset())
                        .with_some_unseen_char(toml_account_cfg.envelope_list_table_unseen_char())
                        .with_some_replied_char(toml_account_cfg.envelope_list_table_replied_char())
                        .with_some_flagged_char(toml_account_cfg.envelope_list_table_flagged_char())
                        .with_some_attachment_char(
                            toml_account_cfg.envelope_list_table_attachment_char(),
                        )
                        .with_some_id_color(toml_account_cfg.envelope_list_table_id_color())
                        .with_some_flags_color(toml_account_cfg.envelope_list_table_flags_color())
                        .with_some_subject_color(
                            toml_account_cfg.envelope_list_table_subject_color(),
                        )
                        .with_some_sender_color(toml_account_cfg.envelope_list_table_sender_color())
                        .with_some_date_color(toml_account_cfg.envelope_list_table_date_color());

                    println!("{table}");
                }
                "read" => {
                    let Some(folder) = folder.as_deref() else {
                        eprintln!("Please select a folder first");
                        continue;
                    };

                    let id = prompt::usize("Select an envelope identifier:", None)?;

                    let emails = backend.get_messages(folder, &Id::single(id)).await?;

                    let mut glue = "";
                    let mut bodies = String::default();

                    for email in emails.to_vec() {
                        bodies.push_str(glue);

                        let tpl = email.to_read_tpl(&account_cfg, |tpl| tpl).await?;
                        bodies.push_str(&tpl);

                        glue = "\n\n";
                    }

                    println!("{bodies}");
                }
                "write" => {
                    let tpl = Message::new_tpl_builder(account_cfg.clone())
                        .build()
                        .await?;

                    editor::edit_tpl_with_editor(account_cfg.clone(), &backend, tpl).await?;
                }
                "reply" => {
                    let Some(folder) = folder.as_deref() else {
                        eprintln!("Please select a folder first");
                        continue;
                    };

                    let id = prompt::usize("Select an envelope identifier:", None)?;
                    let reply_all = prompt::bool("Reply to all recipients?", false)?;

                    let tpl = backend
                        .get_messages(folder, &Id::single(id))
                        .await?
                        .first()
                        .ok_or(eyre!("cannot find message {id}"))?
                        .to_reply_tpl_builder(account_cfg.clone())
                        .with_reply_all(reply_all)
                        .build()
                        .await?;

                    editor::edit_tpl_with_editor(account_cfg.clone(), &backend, tpl).await?;
                }
                "forward" => {
                    let Some(folder) = folder.as_deref() else {
                        eprintln!("Please select a folder first");
                        continue;
                    };

                    let id = prompt::usize("Select an envelope identifier:", None)?;

                    let tpl = backend
                        .get_messages(folder, &Id::single(id))
                        .await?
                        .first()
                        .ok_or(eyre!("cannot find message"))?
                        .to_forward_tpl_builder(account_cfg.clone())
                        .build()
                        .await?;

                    editor::edit_tpl_with_editor(account_cfg.clone(), &backend, tpl).await?;
                }
                "copy" => {
                    let Some(source) = folder.as_deref() else {
                        eprintln!("Please select a folder first");
                        continue;
                    };

                    let folders = backend.list_folders().await?.into_iter().filter_map(|f| {
                        if f.name == source {
                            None
                        } else {
                            Some(f.name)
                        }
                    });

                    let id = prompt::usize("Select an envelope identifier:", None)?;
                    let target = prompt::item("Select a target folder:", folders, None)?;

                    backend
                        .copy_messages(source, &target, &Id::single(id))
                        .await?;
                }
                "move" => {
                    let Some(source) = folder.as_deref() else {
                        eprintln!("Please select a folder first");
                        continue;
                    };

                    let folders = backend.list_folders().await?.into_iter().filter_map(|f| {
                        if f.name == source {
                            None
                        } else {
                            Some(f.name)
                        }
                    });

                    let id = prompt::usize("Select an envelope identifier:", None)?;
                    let target = prompt::item("Select a target folder:", folders, None)?;

                    backend
                        .move_messages(source, &target, &Id::single(id))
                        .await?;
                }
                "delete" => {
                    let Some(folder) = folder.as_deref() else {
                        eprintln!("Please select a folder first");
                        continue;
                    };

                    let id = prompt::usize("Select an envelope identifier:", None)?;

                    backend.delete_messages(folder, &Id::single(id)).await?;
                }
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

struct UnselectedMode(Reedline);

impl UnselectedMode {
    pub fn new(keybinds: KeybindsStyle) -> impl DerefMut<Target = Reedline> {
        let commands = vec!["create".into(), "list".into(), "select".into()];
        let completer = Box::new(DefaultCompleter::new_with_wordlen(commands.clone(), 2));
        let completion_menu = Box::new(ColumnarMenu::default().with_name("completion"));

        let reedline = Reedline::create()
            .with_completer(completer)
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu));

        let reedline = match keybinds {
            KeybindsStyle::Emacs => {
                let mut keybinds = default_emacs_keybindings();

                keybinds.add_binding(
                    KeyModifiers::NONE,
                    KeyCode::Tab,
                    ReedlineEvent::UntilFound(vec![
                        ReedlineEvent::Menu("completion".to_string()),
                        ReedlineEvent::MenuNext,
                    ]),
                );

                reedline.with_edit_mode(Box::new(Emacs::new(keybinds)))
            }
            KeybindsStyle::Vi => {
                let mut keybinds = default_vi_insert_keybindings();

                keybinds.add_binding(
                    KeyModifiers::NONE,
                    KeyCode::Tab,
                    ReedlineEvent::UntilFound(vec![
                        ReedlineEvent::Menu("completion".to_string()),
                        ReedlineEvent::MenuNext,
                    ]),
                );

                reedline
                    .with_edit_mode(Box::new(Vi::new(keybinds, default_vi_normal_keybindings())))
            }
        };

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

pub(crate) fn map_color(color: Color) -> comfy_table::Color {
    match color {
        Color::Reset => comfy_table::Color::Reset,
        Color::Black => comfy_table::Color::Black,
        Color::DarkGrey => comfy_table::Color::DarkGrey,
        Color::Red => comfy_table::Color::Red,
        Color::DarkRed => comfy_table::Color::DarkRed,
        Color::Green => comfy_table::Color::Green,
        Color::DarkGreen => comfy_table::Color::DarkGreen,
        Color::Yellow => comfy_table::Color::Yellow,
        Color::DarkYellow => comfy_table::Color::DarkYellow,
        Color::Blue => comfy_table::Color::Blue,
        Color::DarkBlue => comfy_table::Color::DarkBlue,
        Color::Magenta => comfy_table::Color::Magenta,
        Color::DarkMagenta => comfy_table::Color::DarkMagenta,
        Color::Cyan => comfy_table::Color::Cyan,
        Color::DarkCyan => comfy_table::Color::DarkCyan,
        Color::White => comfy_table::Color::White,
        Color::Grey => comfy_table::Color::Grey,
        Color::Rgb { r, g, b } => comfy_table::Color::Rgb { r, g, b },
        Color::AnsiValue(n) => comfy_table::Color::AnsiValue(n),
    }
}
