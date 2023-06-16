use std::{
    any::Any,
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    path::Path,
};

use crate::app::App;
use crate::traits::EditingPopUp;
use serde::{Deserialize, Serialize};

use tui::{
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Copy)]
pub struct Config {
    pub work_day_hours: f32,
    pub free_holidays: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            work_day_hours: 8.0,
            free_holidays: true,
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> io::Result<Config> {
    match File::open(path) {
        Ok(f) => {
            let file = BufReader::new(f);
            Ok(serde_json::from_reader(file).unwrap_or_default())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e),
    }
}

pub fn store_config<W>(writer: W, config: Config) -> io::Result<()>
where
    W: Write,
{
    let file = BufWriter::new(writer);
    serde_json::to_writer_pretty(file, &config)?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ConfigBeingBuilt {
    pub work_day_hours: String,
    pub free_holidays: String,
    pub selected: ConfigSelected,
    pub editing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigSelected {
    WorkDayHours,
    FreeHolidays,
}

impl ConfigSelected {
    pub fn next(self) -> Self {
        match self {
            Self::WorkDayHours => Self::FreeHolidays,
            Self::FreeHolidays => Self::WorkDayHours,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::FreeHolidays => Self::WorkDayHours,
            Self::WorkDayHours => Self::FreeHolidays,
        }
    }
}

impl ConfigBeingBuilt {
    pub fn new(config: Config) -> Self {
        Self {
            work_day_hours: config.work_day_hours.to_string(),
            free_holidays: config.free_holidays.to_string(),
            selected: ConfigSelected::WorkDayHours,
            editing: true,
        }
    }
}

impl EditingPopUp for ConfigBeingBuilt {
    fn select_next(&mut self) {
        self.selected = self.selected.next();
    }

    fn select_prev(&mut self) {
        self.selected = self.selected.prev();
    }

    fn selected_buf(&mut self) -> &mut String {
        match self.selected {
            ConfigSelected::WorkDayHours => &mut self.work_day_hours,
            ConfigSelected::FreeHolidays => &mut self.free_holidays,
        }
    }

    fn set_editing(&mut self, state: bool) {
        self.editing = state;
    }

    fn is_editing(&self) -> bool {
        self.editing
    }

    fn submit(&self, app: &mut App) -> Result<(), &'static str> {
        app.config = self.try_into()?;
        app.pop_up = None;
        let _ = app.save_to(&app.filename);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn render(&self) -> Vec<tui::widgets::Paragraph<'_>> {
        let mkparagraph = |title, buf, action| {
            Paragraph::new(buf)
                .style(if action == self.selected {
                    let color = if self.editing {
                        Color::Yellow
                    } else {
                        Color::Blue
                    };
                    Style::default().fg(color)
                } else {
                    Style::default()
                })
                .block(Block::default().borders(Borders::ALL).title(title))
        };
        vec![
            mkparagraph(
                "workday hours",
                self.work_day_hours.to_string(),
                ConfigSelected::WorkDayHours,
            ),
            mkparagraph(
                "free holidays",
                self.free_holidays.to_string(),
                ConfigSelected::FreeHolidays,
            ),
        ]
    }

    fn popup_type(&self) -> crate::app::PopUpType {
        crate::app::PopUpType::Config
    }
}

impl TryFrom<&ConfigBeingBuilt> for Config {
    type Error = &'static str;

    fn try_from(builder: &ConfigBeingBuilt) -> Result<Self, Self::Error> {
        let work_day_hours = match builder.work_day_hours.parse::<f32>() {
            Ok(wdh) => {
                if wdh < 0.0 {
                    Err("Work Hours need to be a positive number")
                } else {
                    Ok(wdh)
                }
            }
            Err(_) => Err("Please Provide a number"),
        }?;
        let free_holidays = match builder.free_holidays.parse::<bool>() {
            Ok(fh) => Ok(fh),
            Err(_) => Err("Free Holidays need to be a boolean"),
        }?;
        Ok(Config {
            work_day_hours,
            free_holidays,
        })
    }
}

impl TryFrom<&mut ConfigBeingBuilt> for Config {
    type Error = &'static str;

    fn try_from(builder: &mut ConfigBeingBuilt) -> Result<Self, Self::Error> {
        Config::try_from(&*builder)
    }
}
