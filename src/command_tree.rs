use crate::update::Message;
use crossterm::event::KeyCode;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use std::collections::HashMap;

fn render_help_text(entries: Vec<(String, Vec<(String, String)>)>) -> Text<'static> {
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

#[derive(Debug, Clone)]
pub struct CommandTreeNodeChildren(Vec<(String, HashMap<KeyCode, CommandTreeNode>)>);

impl CommandTreeNodeChildren {
    pub fn get(&self, key_code: &KeyCode) -> Option<&CommandTreeNodeData> {
        let mut merged_map = HashMap::new();
        for (_group_text, help_group) in &self.0 {
            merged_map.extend(help_group)
        }
        merged_map.get(key_code).map(|node| &node.data)
    }

    pub fn get_help_entries(&self) -> Vec<(String, Vec<(String, String)>)> {
        let mut entries = Vec::new();

        for (group_help_text, help_group) in &self.0 {
            let mut help_group_entries: Vec<(String, String)> = help_group
                .iter()
                .map(|(key_code, node)| (format!("{key_code}"), node.help_text.clone()))
                .collect();
            help_group_entries.sort_by(|(a, _), (b, _)| a.cmp(b));
            entries.push((group_help_text.clone(), help_group_entries));
        }

        entries
    }
}

#[derive(Debug, Clone)]
struct CommandTreeNode {
    help_text: String,
    data: CommandTreeNodeData,
}

impl CommandTreeNode {
    fn new(help_text: String, data: CommandTreeNodeData) -> Self {
        Self { help_text, data }
    }
}

#[derive(Debug, Clone)]
pub enum CommandTreeNodeData {
    Children(CommandTreeNodeChildren),
    Action(Message),
}

#[derive(Debug)]
pub struct CommandTree(CommandTreeNodeChildren);

impl CommandTree {
    fn extend(
        &mut self,
        group_help_text: String,
        entries: Vec<(&str, KeyCode, CommandTreeNodeData)>,
    ) {
        let mut help_group = HashMap::new();
        for (help_text, key_code, data) in entries {
            help_group.insert(key_code, CommandTreeNode::new(help_text.to_string(), data));
        }

        self.0.0.push((group_help_text, help_group));
    }

    pub fn get(&self, key_code: &KeyCode) -> Option<&CommandTreeNodeData> {
        self.0.get(key_code)
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

        let mut entries = self.0.get_help_entries();
        entries.push(("Navigation".to_string(), nav_help));
        entries.push(("General".to_string(), general_help));
        render_help_text(entries)
    }

    pub fn new() -> Self {
        let items = vec![
            (
                "Describe change",
                KeyCode::Char('d'),
                CommandTreeNodeData::Action(Message::Describe),
            ),
            (
                "New change",
                KeyCode::Char('n'),
                CommandTreeNodeData::Action(Message::New),
            ),
            (
                "Abandon change",
                KeyCode::Char('a'),
                CommandTreeNodeData::Action(Message::Abandon),
            ),
            (
                "Undo operation",
                KeyCode::Char('u'),
                CommandTreeNodeData::Action(Message::Undo),
            ),
            (
                "Commit change",
                KeyCode::Char('c'),
                CommandTreeNodeData::Action(Message::Commit),
            ),
            (
                "Squash change",
                KeyCode::Char('s'),
                CommandTreeNodeData::Action(Message::Squash),
            ),
            (
                "Edit change",
                KeyCode::Char('e'),
                CommandTreeNodeData::Action(Message::Edit),
            ),
            (
                "Git fetch",
                KeyCode::Char('f'),
                CommandTreeNodeData::Action(Message::Fetch),
            ),
            (
                "Git push",
                KeyCode::Char('p'),
                CommandTreeNodeData::Action(Message::Push),
            ),
            (
                "Set master bookmark",
                KeyCode::Char('m'),
                CommandTreeNodeData::Action(Message::BookmarkSetMaster),
            ),
        ];

        let mut tree = Self(CommandTreeNodeChildren(Vec::new()));
        tree.extend("Commands".to_string(), items);
        tree
    }
}
