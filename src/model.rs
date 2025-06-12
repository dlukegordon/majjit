#[derive(Debug, Default)]
pub struct Model {
    pub counter: i32,
    pub state: State,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Running,
    Quit,
}

#[derive(PartialEq)]
pub enum Message {
    Increment,
    Decrement,
    Reset,
    Quit,
}
