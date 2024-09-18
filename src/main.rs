use std::{
    collections::{HashSet, VecDeque},
    hash::{Hash, Hasher},
};

use color_eyre::Result;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use once_cell::sync::Lazy;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    layout::{Constraint, Layout, Position},
    style::Stylize,
    text::{Line, Text},
    widgets::{List, ListItem},
    DefaultTerminal, Frame,
};

static COMMANDS: Lazy<HashSet<Command>> = Lazy::new(|| {
    HashSet::from_iter([
        Command::new("account").with_children([Command::new("list"), Command::new("doctor")]),
        Command::new("folder").with_children([
            Command::new("add"),
            Command::new("list"),
            Command::new("expunge"),
            Command::new("purge"),
            Command::new("delete"),
        ]),
        Command::new("envelope").with_children([Command::new("list"), Command::new("thread")]),
        Command::new("flag").with_children([
            Command::new("add"),
            Command::new("set"),
            Command::new("remove"),
        ]),
        Command::new("message").with_children([
            Command::new("read"),
            Command::new("thread"),
            Command::new("write"),
            Command::new("reply"),
            Command::new("forward"),
            Command::new("copy"),
            Command::new("move"),
            Command::new("delete"),
        ]),
    ])
});

struct App {
    input: String,
    character_index: usize,
    choices: Vec<String>,
    log: String,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            character_index: 0,
            choices: Vec::new(),
            log: String::new(),
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_message(&mut self) {
        self.input.clear();
        self.reset_cursor();
    }

    fn complete(&mut self) {
        self.choices.clear();

        let mut cmds = COMMANDS.clone();
        let mut names = VecDeque::new();

        for name in self.input.split_whitespace() {
            let cmd = Command::new(name);
            names.push_back(name);

            if let Some(cmd) = cmds.get(&cmd) {
                cmds = cmd.children.clone();
            }
        }

        let matcher = SkimMatcherV2::default();
        let mut matches = HashSet::new();

        let input = names.pop_back().unwrap_or(&self.input);

        for (i, cmd) in cmds.into_iter().enumerate() {
            if input.is_empty() || self.input.ends_with(' ') {
                matches.insert((i as i64, cmd));
            } else if let Some(score) = matcher.fuzzy_match(&cmd.name, input) {
                matches.insert((score, cmd));
            }
        }

        match matches.len() {
            0 => {
                return;
            }
            1 => {
                names.push_back(&matches.iter().next().unwrap().1.name);
                self.input = names.iter().map(|name| name.to_string() + " ").collect();
                self.character_index = self.input.chars().count();
            }
            _ => {
                let mut scores: Vec<_> = matches.iter().cloned().collect();
                scores.sort_by(|(a, _), (b, _)| a.cmp(b));

                for (_, cmd) in scores {
                    self.choices.push(cmd.name)
                }
            }
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Tab => self.complete(),
                    KeyCode::Enter => self.submit_message(),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char(c) => self.enter_char(c),
                    KeyCode::Backspace => self.delete_char(),
                    KeyCode::Left => self.move_cursor_left(),
                    KeyCode::Right => self.move_cursor_right(),
                    _ => {}
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let prompt = "himalaya $ ";

        let layout = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(self.choices.len() as u16),
            Constraint::Length(1),
        ]);

        let [output_area, completion_area, input_area] = layout.areas(frame.area());

        frame.render_widget(Line::raw(&self.log), output_area);

        let items = self
            .choices
            .iter()
            .map(|choice| ListItem::new(Text::raw(choice).cyan()));

        frame.render_widget(List::new(items), completion_area);

        let prompt_layout =
            Layout::horizontal([Constraint::Length(prompt.len() as u16), Constraint::Fill(1)]);

        let [prompt_area, input_area] = prompt_layout.areas(input_area);

        frame.render_widget(Text::raw(prompt).blue(), prompt_area);
        frame.render_widget(&self.input, input_area);

        frame.set_cursor_position(Position::new(
            input_area.x + self.character_index as u16,
            input_area.y,
        ))
    }
}

#[derive(Clone, Debug, Eq)]
struct Command {
    name: String,
    children: HashSet<Command>,
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for Command {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Command {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            children: HashSet::new(),
        }
    }

    fn with_children(mut self, children: impl IntoIterator<Item = Command>) -> Self {
        self.children = children.into_iter().collect();
        self
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}
