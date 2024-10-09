use std::{env, fmt, fs, sync::Arc};

use color_eyre::{eyre::Context, Result};
use email::{
    account::config::AccountConfig,
    flag::{Flag, Flags},
    folder::DRAFTS,
    local_draft_path,
    message::{add::AddMessage, send::SendMessageThenSaveCopy},
    remove_local_draft,
    template::Template,
};
use mml::MmlCompilerBuilder;
use pimalaya_tui::prompt;
use process::SingleCommand;

use crate::backend::Backend;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreEditChoice {
    Edit,
    Discard,
    Quit,
}

impl fmt::Display for PreEditChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Edit => "Edit it",
                Self::Discard => "Discard it",
                Self::Quit => "Quit",
            }
        )
    }
}

static PRE_EDIT_CHOICES: [PreEditChoice; 3] = [
    PreEditChoice::Edit,
    PreEditChoice::Discard,
    PreEditChoice::Quit,
];

pub fn pre_edit() -> Result<PreEditChoice> {
    let user_choice = prompt::item(
        "A draft was found, what would you like to do with it?",
        &PRE_EDIT_CHOICES,
        None,
    )?;

    Ok(user_choice.clone())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PostEditChoice {
    Send,
    Edit,
    LocalDraft,
    RemoteDraft,
    Discard,
}

impl fmt::Display for PostEditChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Send => "Send it",
                Self::Edit => "Edit it again",
                Self::LocalDraft => "Save it as local draft",
                Self::RemoteDraft => "Save it as remote draft",
                Self::Discard => "Discard it",
            }
        )
    }
}

static POST_EDIT_CHOICES: [PostEditChoice; 5] = [
    PostEditChoice::Send,
    PostEditChoice::Edit,
    PostEditChoice::LocalDraft,
    PostEditChoice::RemoteDraft,
    PostEditChoice::Discard,
];

pub fn post_edit() -> Result<PostEditChoice> {
    let user_choice = prompt::item(
        "What would you like to do with this message?",
        &POST_EDIT_CHOICES,
        None,
    )?;

    Ok(user_choice.clone())
}
pub async fn edit_tpl_with_editor(
    #[cfg_attr(not(feature = "pgp"), allow(unused_variables))] config: Arc<AccountConfig>,
    backend: &Backend,
    mut tpl: Template,
) -> Result<()> {
    let draft = local_draft_path();
    if draft.exists() {
        loop {
            match pre_edit() {
                Ok(choice) => match choice {
                    PreEditChoice::Edit => {
                        tpl = open_with_local_draft().await?;
                        break;
                    }
                    PreEditChoice::Discard => {
                        tpl = open_with_tpl(tpl).await?;
                        break;
                    }
                    PreEditChoice::Quit => return Ok(()),
                },
                Err(err) => {
                    println!("{}", err);
                    continue;
                }
            }
        }
    } else {
        tpl = open_with_tpl(tpl).await?;
    }

    loop {
        match post_edit() {
            Ok(PostEditChoice::Send) => {
                println!("Sending email…");

                #[allow(unused_mut)]
                let mut compiler = MmlCompilerBuilder::new();

                #[cfg(feature = "pgp")]
                compiler.set_some_pgp(config.pgp.clone());

                let email = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

                backend.send_message_then_save_copy(&email).await?;

                remove_local_draft()?;
                println!("Done!");
                break;
            }
            Ok(PostEditChoice::Edit) => {
                tpl = open_with_tpl(tpl).await?;
                continue;
            }
            Ok(PostEditChoice::LocalDraft) => {
                println!("Email successfully saved locally");
                break;
            }
            Ok(PostEditChoice::RemoteDraft) => {
                #[allow(unused_mut)]
                let mut compiler = MmlCompilerBuilder::new();

                #[cfg(feature = "pgp")]
                compiler.set_some_pgp(config.pgp.clone());

                let email = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

                backend
                    .add_message_with_flags(
                        DRAFTS,
                        &email,
                        &Flags::from_iter([Flag::Seen, Flag::Draft]),
                    )
                    .await?;
                remove_local_draft()?;
                println!("Email successfully saved to drafts");
                break;
            }
            Ok(PostEditChoice::Discard) => {
                remove_local_draft()?;
                break;
            }
            Err(err) => {
                println!("{}", err);
                continue;
            }
        }
    }

    Ok(())
}

pub async fn open_with_local_draft() -> Result<Template> {
    let path = local_draft_path();
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;
    open_with_tpl(content.into()).await
}

pub async fn open_with_tpl(tpl: Template) -> Result<Template> {
    let path = local_draft_path();

    tracing::debug!("create draft");
    fs::write(&path, tpl.as_bytes()).context(format!("cannot write local draft at {:?}", path))?;

    tracing::debug!("open editor");
    let editor = env::var("EDITOR").context("cannot get editor from env var")?;
    SingleCommand::from(format!("{editor} {}", &path.to_string_lossy()))
        .with_output_piped(false)
        .run()
        .await
        .context("cannot launch editor")?;

    tracing::debug!("read draft");
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;

    Ok(content.into())
}
