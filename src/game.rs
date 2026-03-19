#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Running,
    Paused,
}

pub struct GameState {
    tick_count: u64,
    state: RunState,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            tick_count: 0,
            state: RunState::Running,
        }
    }

    pub fn tick(&mut self) {
        if self.state == RunState::Running {
            self.tick_count += 1;
        }
    }

    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            RunState::Running => RunState::Paused,
            RunState::Paused => RunState::Running,
        };
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn run_state(&self) -> RunState {
        self.state
    }
}
