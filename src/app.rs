use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::game::{Direction, GameState};
use crate::render::{self, board_size_for_terminal};

const TICK_RATE: Duration = Duration::from_millis(160);

pub struct App {
    game: GameState,
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            game: GameState::new(),
            should_quit: false,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = setup_terminal()?;
        self.resize_game_to_terminal(terminal.size()?.into());
        let result = self.run_loop(&mut terminal);
        restore_terminal()?;
        result
    }

    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let mut last_tick = Instant::now();

        while !self.should_quit {
            terminal.draw(|frame| render::draw(frame, &self.game))?;

            let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                self.handle_event(event::read()?)?;
            }

            if last_tick.elapsed() >= TICK_RATE {
                self.game.tick();
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    fn resize_game_to_terminal(&mut self, area: Rect) {
        let (width, height) = board_size_for_terminal(area.width, area.height);
        self.game.restart_with_board_size(width, height);
    }

    fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char(' ') => self.game.toggle_pause(),
                    KeyCode::Char('r') => self.game.restart(),
                    KeyCode::Up | KeyCode::Char('w') => self.game.set_direction(Direction::Up),
                    KeyCode::Down | KeyCode::Char('s') => self.game.set_direction(Direction::Down),
                    KeyCode::Left | KeyCode::Char('a') => self.game.set_direction(Direction::Left),
                    KeyCode::Right | KeyCode::Char('d') => self.game.set_direction(Direction::Right),
                    _ => {}
                }
            }
            Event::Resize(width, height) => {
                let (board_width, board_height) = board_size_for_terminal(width, height);
                self.game.restart_with_board_size(board_width, board_height);
            }
            _ => {}
        }

        Ok(())
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
