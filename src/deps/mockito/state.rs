use super::Mock;

pub struct State {
    pub mocks: Vec<Mock>,
}

impl State {
    pub fn new() -> Self {
        State {
            mocks: vec![],
        }
    }
}
