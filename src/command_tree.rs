use crate::update::Message;
use crossterm::event::KeyCode;
use indexmap::IndexMap;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use std::collections::HashMap;

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

    pub fn get_node_mut(&mut self, key_code: &KeyCode) -> Option<&mut CommandTreeNode> {
        self.nodes.get_mut(key_code)
    }

    pub fn get_help_entries(&self) -> HelpEntries {
        let mut help = self.help.clone();

        for (_, entries) in help.iter_mut() {
            entries.sort();
        }

        help
    }

    pub fn get_help(&self) -> Text<'static> {
        let entries = self.get_help_entries();
        render_help_text(entries)
    }

    pub fn add_child(
        &mut self,
        help_group_text: &str,
        help_text: &str,
        key_code: KeyCode,
        node: CommandTreeNode,
    ) {
        self.nodes.insert(key_code, node);
        let help_group = self.help.entry(help_group_text.to_string()).or_default();
        help_group.push((key_code.to_string(), help_text.to_string()))
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

    fn add_children(&mut self, entries: Vec<(&str, &str, Vec<KeyCode>, CommandTreeNode)>) {
        for (help_group_text, help_text, key_codes, node) in entries {
            let (last_key, rest_keys) = key_codes.split_last().unwrap();
            let dest_node = self.get_node_mut(rest_keys).unwrap();
            let children = match dest_node {
                CommandTreeNode::Action(_) => unreachable!(),
                CommandTreeNode::Children(children) => children,
            };
            children.add_child(help_group_text, help_text, *last_key, node)
        }
    }

    pub fn get_node(&self, key_codes: &[KeyCode]) -> Option<&CommandTreeNode> {
        let mut node = &self.0;

        for key_code in key_codes {
            let children = match node {
                CommandTreeNode::Action(_) => return None,
                CommandTreeNode::Children(children) => children,
            };
            node = children.get_node(key_code)?;
        }

        Some(node)
    }

    fn get_node_mut(&mut self, key_codes: &[KeyCode]) -> Option<&mut CommandTreeNode> {
        let mut node = &mut self.0;

        for key_code in key_codes {
            let children = match node {
                CommandTreeNode::Action(_) => return None,
                CommandTreeNode::Children(children) => children,
            };
            node = children.get_node_mut(key_code)?;
        }

        Some(node)
    }

    pub fn get_help(&self) -> Text<'static> {
        let nav_help = [
            ("Enter", "Show diff"),
            ("Tab ", "Toggle folding"),
            ("PgDn", "Move down page"),
            ("PgUp", "Move up page"),
            ("j/🠋 ", "Move down"),
            ("k/🠉 ", "Move up"),
            ("l/🠊 ", "Next sibling"),
            ("h/🠈 ", "Prev sibling"),
            ("K", "Select parent"),
            ("@", "Select @ change"),
        ]
        .iter()
        .map(|(key, help)| (key.to_string(), help.to_string()))
        .collect();

        let general_help = [
            ("Ctrl-r", "Refresh log tree"),
            ("Esc", "Clear app state"),
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
                "Commands",
                "Abandon change",
                vec![KeyCode::Char('a')],
                CommandTreeNode::new_children(),
            ),
            (
                "Abandon",
                "Selected change",
                vec![KeyCode::Char('a'), KeyCode::Char('a')],
                CommandTreeNode::Action(Message::Abandon),
            ),
            (
                "Commands",
                "Bookmark",
                vec![KeyCode::Char('b')],
                CommandTreeNode::new_children(),
            ),
            (
                "Bookmark",
                "Set",
                vec![KeyCode::Char('b'), KeyCode::Char('s')],
                CommandTreeNode::new_children(),
            ),
            (
                "Bookmark set",
                "Master for selected change",
                vec![KeyCode::Char('b'), KeyCode::Char('s'), KeyCode::Char('m')],
                CommandTreeNode::Action(Message::BookmarkSetMaster),
            ),
            (
                "Commands",
                "Commit change",
                vec![KeyCode::Char('c')],
                CommandTreeNode::new_children(),
            ),
            (
                "Commit",
                "Selected change",
                vec![KeyCode::Char('c'), KeyCode::Char('c')],
                CommandTreeNode::Action(Message::Commit),
            ),
            (
                "Commands",
                "Describe change",
                vec![KeyCode::Char('d')],
                CommandTreeNode::new_children(),
            ),
            (
                "Describe",
                "Selected change",
                vec![KeyCode::Char('d'), KeyCode::Char('d')],
                CommandTreeNode::Action(Message::Describe),
            ),
            (
                "Commands",
                "Edit change",
                vec![KeyCode::Char('e')],
                CommandTreeNode::new_children(),
            ),
            (
                "Edit",
                "Selected change",
                vec![KeyCode::Char('e'), KeyCode::Char('e')],
                CommandTreeNode::Action(Message::Edit),
            ),
            (
                "Commands",
                "Git commands",
                vec![KeyCode::Char('g')],
                CommandTreeNode::new_children(),
            ),
            (
                "Git",
                "Fetch",
                vec![KeyCode::Char('g'), KeyCode::Char('f')],
                CommandTreeNode::Action(Message::GitFetch),
            ),
            (
                "Git",
                "Push",
                vec![KeyCode::Char('g'), KeyCode::Char('p')],
                CommandTreeNode::Action(Message::GitPush),
            ),
            (
                "Commands",
                "New change",
                vec![KeyCode::Char('n')],
                CommandTreeNode::new_children(),
            ),
            (
                "New",
                "After selected change",
                vec![KeyCode::Char('n'), KeyCode::Char('n')],
                CommandTreeNode::Action(Message::New),
            ),
            (
                "New",
                "Before selected change",
                vec![KeyCode::Char('n'), KeyCode::Char('b')],
                CommandTreeNode::Action(Message::NewBefore),
            ),
            (
                "Commands",
                "Restore change",
                vec![KeyCode::Char('r')],
                CommandTreeNode::new_children(),
            ),
            (
                "Restore",
                "Selected change",
                vec![KeyCode::Char('r'), KeyCode::Char('r')],
                CommandTreeNode::Action(Message::Restore),
            ),
            (
                "Commands",
                "Squash change",
                vec![KeyCode::Char('s')],
                CommandTreeNode::new_children(),
            ),
            (
                "Squash",
                "Selected change into parent",
                vec![KeyCode::Char('s'), KeyCode::Char('s')],
                CommandTreeNode::Action(Message::Squash),
            ),
            (
                "Commands",
                "Undo operation",
                vec![KeyCode::Char('u')],
                CommandTreeNode::new_children(),
            ),
            (
                "Undo",
                "Last operation",
                vec![KeyCode::Char('u'), KeyCode::Char('u')],
                CommandTreeNode::Action(Message::Undo),
            ),
        ];

        let mut tree = Self(CommandTreeNode::new_children());
        tree.add_children(items);
        tree
    }
}

fn render_help_text(entries: HelpEntries) -> Text<'static> {
    const COL_WIDTH: usize = 26;

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

pub fn display_error_lines(info_list: &mut Option<Text<'static>>, key_code: &KeyCode) {
    let error_line = Line::from(vec![
        Span::styled(" Unbound suffix: ", Style::default().fg(Color::Red)),
        Span::raw("'"),
        Span::styled(format!("{key_code}"), Style::default().fg(Color::Green)),
        Span::raw("'"),
    ]);
    match info_list {
        None => {
            *info_list = Some(error_line.into());
        }
        Some(info_list) => {
            let add_blank_line = info_list.lines.first().unwrap().spans[0] != error_line.spans[0];
            if info_list.lines.last().unwrap().spans[0] == error_line.spans[0] {
                info_list.lines.pop();
                info_list.lines.pop();
            }

            if add_blank_line {
                info_list.lines.push(Line::from(vec![]));
            }
            info_list.lines.push(error_line);
        }
    }
}
