use ratatui::widgets::ListState;

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Running,
    Quit,
}

#[derive(Debug)]
pub struct Model<'a> {
    pub state: State,
    pub commits: Vec<&'a str>,
    pub commit_list_state: ListState,
}

impl<'a> Model<'a> {
    pub fn new(list: Vec<&'a str>) -> Self {
        let mut commit_list_state = ListState::default();
        commit_list_state.select(Some(0));

        Self {
            state: State::Running,
            commits: list,
            commit_list_state,
        }
    }
}
