mod app;
mod combo_buffer;
mod selected_vec;
mod ui;
mod util;

use combo_buffer::ComboBuffer;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::{
    env::args,
    io::{self, Stdout},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use crate::app::App;

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn main() -> anyhow::Result<()> {
    let path = match args().nth(1) {
        Some(filename) => filename,
        None => {
            println!("Usage: {} [FILE]", args().next().unwrap());
            std::process::exit(1)
        }
    };
    let activities = app::load_activities(&path)?;
    let mut terminal = setup_terminal()?;
    let mut app = App::new(path, activities);
    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    let mut combo_buffer = ComboBuffer::default();
    let mut error = None;
    loop {
        terminal.draw(|f| ui::ui(f, app, &mut error))?;
        error = None;
        if let Event::Key(key) = event::read()? {
            if let KeyCode::Char('q') = key.code {
                if !app.editing() {
                    return Ok(());
                }
            }
            match app.new_activity_mut() {
                Some(new) => {
                    combo_buffer.clear();
                    if new.editing {
                        match key.code {
                            KeyCode::Char(c) => new.selected_buf().push(c),
                            KeyCode::Backspace => {
                                new.selected_buf().pop();
                            }
                            KeyCode::Tab => new.select_next(),
                            KeyCode::BackTab => new.select_prev(),
                            KeyCode::Esc => new.editing = false,
                            KeyCode::Enter => {
                                if let Err(msg) = app.submit_activity() {
                                    error = Some(msg)
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('i') => new.editing = true,
                            KeyCode::Char('k') => new.select_prev(),
                            KeyCode::Char('j') => new.select_next(),
                            KeyCode::Enter => {
                                if let Err(msg) = app.submit_activity() {
                                    error = Some(msg)
                                }
                            }
                            _ => {}
                        }
                    }
                }
                None => {
                    match key.code {
                        KeyCode::Char('k') => app.previous(),
                        KeyCode::Char('j') => app.next(),
                        KeyCode::Char('s') => app.toggle_stats(),
                        KeyCode::Char('o') => app.create_new_activity(),
                        KeyCode::Char('u') => app.undo(),
                        KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => app.redo(),
                        KeyCode::Char('e') => app.edit_activity(),
                        KeyCode::Char('G') => app.select_last(),
                        _ => {}
                    }
                    if let Some(combo) = combo_buffer.combo(key.code) {
                        match combo {
                            combo_buffer::Combo::Delete => {
                                app.delete_activity();
                            }
                            combo_buffer::Combo::SelectFirst => {
                                app.select_first();
                            }
                        }
                    }
                }
            }
        }
    }
}
