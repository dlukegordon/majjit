use crate::update::Message;
use crossterm::event::KeyCode;
use indexmap::IndexMap;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use std::collections::HashMap;

fn render_help_text(entries: HelpEntries) -> Text<'static> {
    const COL_WIDTH: usize = 28;

    // Get lines for each column
    let columns: Vec<Vec<Line>> = entries
        .into_iter()
        .map(|(group_help_text, help_group)| {
            let mut col_lines = Vec::new();
            col_lines.push(Line::from(vec![Span::styled(
                format!("{group_help_text:COL_WIDTH$}"),
                Style::default().fg(Color::Blue),
            )]));
            col_lines.extend(
                help_group
                    .into_iter()
                    .map(|(key, help)| {
                        let mut num_cols = key.len() + 1 + help.len();
                        if !key.is_ascii() {
                            num_cols -= 3;
                        }
                        let padding = " ".repeat(COL_WIDTH.saturating_sub(num_cols));
                        Line::from(vec![
                            Span::styled(key, Style::default().fg(Color::Green)),
                            Span::raw(" "),
                            Span::raw(help),
                            Span::raw(padding),
                        ])
                    })
                    .collect::<Vec<_>>(),
            );
            col_lines
        })
        .collect();

    // Render the columns
    let num_rows = columns.iter().map(|c| c.len()).max().unwrap();
    let lines: Vec<Line> = (0..num_rows)
        .map(|i| {
            let mut spans: Vec<Span> = vec![Span::raw(" ")];

            for col in &columns {
                let empty_line = Line::from(Span::raw(" ".repeat(COL_WIDTH)));
                let col_line = col.get(i).unwrap_or(&empty_line).clone();
                spans.extend(col_line.spans)
            }

            Line::from(spans)
        })
        .collect();

    lines.into()
}

type HelpEntries = IndexMap<String, Vec<(String, String)>>;

#[derive(Debug, Clone)]
pub struct CommandTreeNodeChildren {
    nodes: HashMap<KeyCode, CommandTreeNode>,
    help: HelpEntries,
}

impl CommandTreeNodeChildren {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            help: IndexMap::new(),
        }
    }

    pub fn get_node(&self, key_code: &KeyCode) -> Option<&CommandTreeNode> {
        self.nodes.get(key_code)
    }

    pub fn get_help_entries(&self) -> HelpEntries {
        let mut help = self.help.clone();

        for (_, entries) in help.iter_mut() {
            entries.sort();
        }

        help
    }

    pub fn add_child(
        &mut self,
        help_group_text: &str,
        help_text: String,
        key_code: KeyCode,
        node: CommandTreeNode,
    ) {
        self.nodes.insert(key_code, node);
        let help_group = self.help.entry(help_group_text.to_string()).or_default();
        help_group.push((key_code.to_string(), help_text))
    }
}

#[derive(Debug, Clone)]
pub enum CommandTreeNode {
    Children(CommandTreeNodeChildren),
    Action(Message),
}

impl CommandTreeNode {
    pub fn new_children() -> Self {
        Self::Children(CommandTreeNodeChildren::new())
    }
}

#[derive(Debug)]
pub struct CommandTree(CommandTreeNode);

impl CommandTree {
    fn children(&self) -> &CommandTreeNodeChildren {
        match &self.0 {
            CommandTreeNode::Action(_) => unreachable!(),
            CommandTreeNode::Children(children) => children,
        }
    }

    fn children_mut(&mut self) -> &mut CommandTreeNodeChildren {
        match &mut self.0 {
            CommandTreeNode::Action(_) => unreachable!(),
            CommandTreeNode::Children(children) => children,
        }
    }

    fn add_children(
        &mut self,
        help_group_text: &str,
        entries: Vec<(&str, KeyCode, CommandTreeNode)>,
    ) {
        for (help_text, key_code, node) in entries {
            self.children_mut()
                .add_child(help_group_text, help_text.to_string(), key_code, node);
        }
    }

    pub fn get_node(&self, key_codes: &[KeyCode]) -> Option<&CommandTreeNode> {
        self.children().get_node(&key_codes[0])
    }

    pub fn get_help(&self) -> Text<'static> {
        let nav_help = [
            ("PgDn", "Scroll down page"),
            ("PgUp", "Scroll up page"),
            ("j/🠋 ", "Move down"),
            ("k/🠉 ", "Move up"),
            ("l/🠊 ", "Next sibling"),
            ("h/🠈 ", "Prev sibling"),
            ("K", "Select parent"),
            ("@", "Select @ change"),
            ("Ent", "Show diff"),
            ("Tab", "Toggle folding"),
        ]
        .iter()
        .map(|(key, help)| (key.to_string(), help.to_string()))
        .collect();

        let general_help = [
            ("Ctrl-r", "Refresh log tree"),
            ("Esc", "Clear info popup"),
            ("i", "Toggle --ignore-immutable"),
            ("?", "Show help"),
            ("q", "Quit"),
        ]
        .iter()
        .map(|(key, help)| (key.to_string(), help.to_string()))
        .collect();

        let mut entries = self.children().get_help_entries();
        entries.insert("Navigation".to_string(), nav_help);
        entries.insert("General".to_string(), general_help);
        render_help_text(entries)
    }

    pub fn new() -> Self {
        let items = vec![
            (
                "Describe change",
                KeyCode::Char('d'),
                CommandTreeNode::Action(Message::Describe),
            ),
            (
                "New change",
                KeyCode::Char('n'),
                CommandTreeNode::Action(Message::New),
            ),
            (
                "Abandon change",
                KeyCode::Char('a'),
                CommandTreeNode::Action(Message::Abandon),
            ),
            (
                "Undo operation",
                KeyCode::Char('u'),
                CommandTreeNode::Action(Message::Undo),
            ),
            (
                "Commit change",
                KeyCode::Char('c'),
                CommandTreeNode::Action(Message::Commit),
            ),
            (
                "Squash change",
                KeyCode::Char('s'),
                CommandTreeNode::Action(Message::Squash),
            ),
            (
                "Edit change",
                KeyCode::Char('e'),
                CommandTreeNode::Action(Message::Edit),
            ),
            (
                "Git fetch",
                KeyCode::Char('f'),
                CommandTreeNode::Action(Message::Fetch),
            ),
            (
                "Git push",
                KeyCode::Char('p'),
                CommandTreeNode::Action(Message::Push),
            ),
            (
                "Set master bookmark",
                KeyCode::Char('m'),
                CommandTreeNode::Action(Message::BookmarkSetMaster),
            ),
        ];

        let mut tree = Self(CommandTreeNode::new_children());
        tree.add_children("Commands", items);
        tree
    }
}
