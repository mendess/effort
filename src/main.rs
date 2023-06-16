mod app;
mod combo_buffer;
mod selected_vec;
mod traits;
mod ui;
mod util;

use app::PopUp;
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
    let export = args()
        .nth(2)
        .filter(|s| s == "-e" || s == "--export")
        .is_some();

    let mut app = App::load(path)?;
    if export {
        match app.export() {
            Ok(()) => println!("exported!"),
            Err(e) => println!("failed to export: {:?}", e),
        }
    } else {
        let mut terminal = setup_terminal()?;
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
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    let mut combo_buffer = ComboBuffer::default();
    let mut info_popup = None;
    loop {
        terminal.draw(|f| ui::ui(f, app, &info_popup))?;
        info_popup = None;
        if let Event::Key(key) = event::read()? {
            if let KeyCode::Char('q') = key.code {
                if !app.editing() {
                    return Ok(());
                }
            }
            let n_days_off = app.n_days_off();
            let n_holidays = app.n_holidays();
            dbg!(n_holidays);
            match app.pop_up_mut() {
                Some(PopUp::EditingPopUp(new)) => {
                    combo_buffer.reset();
                    if new.is_editing() {
                        match key.code {
                            KeyCode::Char(c) => new.selected_buf().push(c),
                            KeyCode::Backspace => {
                                new.selected_buf().pop();
                            }
                            KeyCode::Tab => new.select_next(),
                            KeyCode::BackTab => new.select_prev(),
                            KeyCode::Esc => new.set_editing(false),
                            KeyCode::Enter => {
                                if let Err(msg) = app.submit() {
                                    info_popup = Some(Err(msg.into()))
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('i') => new.set_editing(true),
                            KeyCode::Char('k') => new.select_prev(),
                            KeyCode::Char('j') => new.select_next(),
                            KeyCode::Esc => app.cancel_edit(),
                            KeyCode::Enter => {
                                if let Err(msg) = app.submit() {
                                    info_popup = Some(Err(msg.into()))
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Some(app::PopUp::DaysOff {
                    selected,
                    new_day_off,
                }) => {
                    if let Some(new_day_off) = new_day_off {
                        match key.code {
                            KeyCode::Char(c) => new_day_off.push(c),
                            KeyCode::Backspace => {
                                new_day_off.pop();
                            }
                            KeyCode::Enter => {
                                if let Err(msg) = app.submit_new_day_off() {
                                    info_popup = Some(Err(msg.into()))
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('k') => *selected = selected.saturating_sub(1),
                            KeyCode::Char('j') => *selected = (*selected + 1) % n_days_off,
                            KeyCode::Char('o') => *new_day_off = Some(String::new()),
                            KeyCode::Char('f') | KeyCode::Esc => app.hide_days_off(),
                            _ => {}
                        }
                    }
                }
                Some(app::PopUp::Holidays {
                    selected,
                    new_holiday,
                }) => {
                    if let Some(new_holiday) = new_holiday {
                        match key.code {
                            KeyCode::Char(c) => new_holiday.push(c),
                            KeyCode::Backspace => {
                                new_holiday.pop();
                            }
                            KeyCode::Enter => {
                                if let Err(msg) = app.submit_new_holiday() {
                                    info_popup = Some(Err(msg.into()))
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('k') => *selected = selected.saturating_sub(1),
                            KeyCode::Char('j') => *selected = (*selected + 1) % n_holidays,
                            KeyCode::Char('o') => *new_holiday = Some(String::new()),
                            KeyCode::Char('h') | KeyCode::Esc => app.hide_holidays(),
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
                        KeyCode::Char('?') => app.edit_config(),
                        KeyCode::Char('u') => app.undo(),
                        KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => app.redo(),
                        KeyCode::Char('e') => app.edit_activity(),
                        KeyCode::Char('G') => app.select_last(),
                        KeyCode::Char('f') => app.show_days_off(),
                        KeyCode::Char('h') => app.show_holidays(),
                        KeyCode::Char('p') => {
                            if let Err(msg) = app.paste() {
                                info_popup = Some(Err(msg.into()))
                            }
                        }
                        _ => {}
                    }
                    if let Some(combo) = combo_buffer.combo(key.code) {
                        match combo {
                            combo_buffer::ComboAction::Delete => {
                                app.delete_activity();
                            }
                            combo_buffer::ComboAction::SelectFirst => {
                                app.select_first();
                            }
                            combo_buffer::ComboAction::Save => {
                                match app.save() {
                                    Ok(_) => info_popup = Some(Ok("saved successfully".into())),
                                    Err(e) => {
                                        info_popup =
                                            Some(Err(format!("failed to save: {}", e).into()))
                                    }
                                };
                            }
                            combo_buffer::ComboAction::Yank => {
                                info_popup = if app.yank_selected() {
                                    Some(Ok("yanked!".into()))
                                } else {
                                    Some(Err("nothing selected".into()))
                                };
                            }
                        }
                    }
                }
            }
        }
    }
}
