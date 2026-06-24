use ratatui::widgets::ListState;

pub struct RichListState<T> {
    pub rich_state: Vec<T>,
    pub state: ListState,
}

impl<T> Default for RichListState<T> {
    fn default() -> Self {
        Self {
            rich_state: Vec::new(),
            state: ListState::default().with_selected(Some(0)),
        }
    }
}

impl<T> RichListState<T> {
    pub fn new(rich_state: Vec<T>) -> Self {
        Self {
            rich_state,
            state: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn selected(&self) -> Option<&T> {
        let index = self.state.selected();

        match index {
            Some(index) => self.rich_state.get(index),
            None => None,
        }
    }

    pub fn select_next(&mut self) {
        self.state.select_next();
    }

    pub fn select_previous(&mut self) {
        self.state.select_previous();
    }
}
