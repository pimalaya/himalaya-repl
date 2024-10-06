use std::{collections::HashSet, fmt, ops::Deref, sync::Arc};

use color_eyre::Result;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Row, Table};
use crossterm::style::Color;
use email::account::config::AccountConfig;
use serde::{Deserialize, Serialize};

use crate::{id_mapper::IdMapper, map_color};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Mailbox {
    pub name: Option<String>,
    pub addr: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelope {
    pub id: String,
    pub flags: Flags,
    pub subject: String,
    pub from: Mailbox,
    pub to: Mailbox,
    pub date: String,
    pub has_attachment: bool,
}

impl Envelope {
    fn to_row(&self, config: &ListEnvelopesTableConfig) -> Row {
        let mut all_attributes = vec![];

        let unseen = !self.flags.contains(&Flag::Seen);
        if unseen {
            all_attributes.push(Attribute::Bold)
        }

        let flags = {
            let mut flags = String::new();

            flags.push(config.flagged_char(self.flags.contains(&Flag::Flagged)));
            flags.push(config.unseen_char(unseen));
            flags.push(config.attachment_char(self.has_attachment));
            flags.push(config.replied_char(self.flags.contains(&Flag::Answered)));

            flags
        };

        let mut row = Row::new();
        row.max_height(1);

        row.add_cell(
            Cell::new(&self.id)
                .add_attributes(all_attributes.clone())
                .fg(config.id_color()),
        )
        .add_cell(
            Cell::new(flags)
                .add_attributes(all_attributes.clone())
                .fg(config.flags_color()),
        )
        .add_cell(
            Cell::new(&self.subject)
                .add_attributes(all_attributes.clone())
                .fg(config.subject_color()),
        )
        .add_cell(
            Cell::new(if let Some(name) = &self.from.name {
                name
            } else {
                &self.from.addr
            })
            .add_attributes(all_attributes.clone())
            .fg(config.sender_color()),
        )
        .add_cell(
            Cell::new(&self.date)
                .add_attributes(all_attributes)
                .fg(config.date_color()),
        );

        row
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelopes(Vec<Envelope>);

impl Envelopes {
    pub fn try_from_lib(
        config: Arc<AccountConfig>,
        id_mapper: &IdMapper,
        envelopes: email::envelope::Envelopes,
    ) -> Result<Envelopes> {
        let envelopes = envelopes
            .iter()
            .map(|envelope| {
                Ok(Envelope {
                    id: id_mapper.get_or_create_alias(&envelope.id)?,
                    flags: envelope.flags.clone().into(),
                    subject: envelope.subject.clone(),
                    from: Mailbox {
                        name: envelope.from.name.clone(),
                        addr: envelope.from.addr.clone(),
                    },
                    to: Mailbox {
                        name: envelope.to.name.clone(),
                        addr: envelope.to.addr.clone(),
                    },
                    date: envelope.format_date(&config),
                    has_attachment: envelope.has_attachment,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Envelopes(envelopes))
    }
}

impl Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Envelopes {
    pub fn new(envelopes: Vec<Envelope>) -> Self {
        Self(envelopes)
    }
}

pub struct EnvelopesTable {
    envelopes: Envelopes,
    width: Option<u16>,
    config: ListEnvelopesTableConfig,
}

impl EnvelopesTable {
    pub fn with_some_width(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }

    pub fn with_some_preset(mut self, preset: Option<String>) -> Self {
        self.config.preset = preset;
        self
    }

    pub fn with_some_unseen_char(mut self, char: Option<char>) -> Self {
        self.config.unseen_char = char;
        self
    }

    pub fn with_some_replied_char(mut self, char: Option<char>) -> Self {
        self.config.replied_char = char;
        self
    }

    pub fn with_some_flagged_char(mut self, char: Option<char>) -> Self {
        self.config.flagged_char = char;
        self
    }

    pub fn with_some_attachment_char(mut self, char: Option<char>) -> Self {
        self.config.attachment_char = char;
        self
    }

    pub fn with_some_id_color(mut self, color: Option<Color>) -> Self {
        self.config.id_color = color;
        self
    }

    pub fn with_some_flags_color(mut self, color: Option<Color>) -> Self {
        self.config.flags_color = color;
        self
    }

    pub fn with_some_subject_color(mut self, color: Option<Color>) -> Self {
        self.config.subject_color = color;
        self
    }

    pub fn with_some_sender_color(mut self, color: Option<Color>) -> Self {
        self.config.sender_color = color;
        self
    }

    pub fn with_some_date_color(mut self, color: Option<Color>) -> Self {
        self.config.date_color = color;
        self
    }
}

impl From<Envelopes> for EnvelopesTable {
    fn from(envelopes: Envelopes) -> Self {
        Self {
            envelopes,
            width: None,
            config: Default::default(),
        }
    }
}

impl fmt::Display for EnvelopesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(self.config.preset())
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("FLAGS"),
                Cell::new("SUBJECT"),
                Cell::new("FROM"),
                Cell::new("DATE"),
            ]))
            .add_rows(self.envelopes.iter().map(|env| env.to_row(&self.config)));

        if let Some(width) = self.width {
            table.set_width(width);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListEnvelopesTableConfig {
    pub preset: Option<String>,

    pub unseen_char: Option<char>,
    pub replied_char: Option<char>,
    pub flagged_char: Option<char>,
    pub attachment_char: Option<char>,

    pub id_color: Option<Color>,
    pub flags_color: Option<Color>,
    pub subject_color: Option<Color>,
    pub sender_color: Option<Color>,
    pub date_color: Option<Color>,
}

impl ListEnvelopesTableConfig {
    pub fn preset(&self) -> &str {
        self.preset.as_deref().unwrap_or(presets::ASCII_MARKDOWN)
    }

    pub fn replied_char(&self, replied: bool) -> char {
        if replied {
            self.replied_char.unwrap_or('R')
        } else {
            ' '
        }
    }

    pub fn flagged_char(&self, flagged: bool) -> char {
        if flagged {
            self.flagged_char.unwrap_or('!')
        } else {
            ' '
        }
    }

    pub fn attachment_char(&self, attachment: bool) -> char {
        if attachment {
            self.attachment_char.unwrap_or('@')
        } else {
            ' '
        }
    }

    pub fn unseen_char(&self, unseen: bool) -> char {
        if unseen {
            self.unseen_char.unwrap_or('*')
        } else {
            ' '
        }
    }

    pub fn id_color(&self) -> comfy_table::Color {
        map_color(self.id_color.unwrap_or(Color::Red))
    }

    pub fn flags_color(&self) -> comfy_table::Color {
        map_color(self.flags_color.unwrap_or(Color::Reset))
    }

    pub fn subject_color(&self) -> comfy_table::Color {
        map_color(self.subject_color.unwrap_or(Color::Green))
    }

    pub fn sender_color(&self) -> comfy_table::Color {
        map_color(self.sender_color.unwrap_or(Color::Blue))
    }

    pub fn date_color(&self) -> comfy_table::Color {
        map_color(self.date_color.unwrap_or(Color::DarkYellow))
    }
}

/// Represents the flag variants.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize)]
pub enum Flag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Custom(String),
}

impl From<&email::flag::Flag> for Flag {
    fn from(flag: &email::flag::Flag) -> Self {
        use email::flag::Flag::*;
        match flag {
            Seen => Flag::Seen,
            Answered => Flag::Answered,
            Flagged => Flag::Flagged,
            Deleted => Flag::Deleted,
            Draft => Flag::Draft,
            Custom(flag) => Flag::Custom(flag.clone()),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Flags(pub HashSet<Flag>);

impl Deref for Flags {
    type Target = HashSet<Flag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<email::flag::Flags> for Flags {
    fn from(flags: email::flag::Flags) -> Self {
        Flags(flags.iter().map(Flag::from).collect())
    }
}
