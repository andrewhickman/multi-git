use std::io::{self, Write as _};

use crossterm::cursor::MoveRight;
use crossterm::style::{Attribute, SetAttribute};

#[derive(Clone, Debug)]
pub struct ProgressBar {
    progress: f64,
}

impl ProgressBar {
    pub fn new() -> Self {
        ProgressBar { progress: 0.0 }
    }

    pub fn write(&self, stdout: &mut io::StdoutLock, width: u16) -> crossterm::Result<()> {
        let bar_width = width.saturating_sub(2) as usize;
        let progress_width = (bar_width as f64 * self.progress) as usize;

        crossterm::queue!(stdout, SetAttribute(Attribute::Dim))?;
        write!(stdout, "[")?;
        stdout.flush()?;

        crossterm::queue!(stdout, SetAttribute(Attribute::Bold))?;
        write!(stdout, "{:=>width$}", ">", width = progress_width)?;
        crossterm::queue!(stdout, SetAttribute(Attribute::Reset))?;

        if progress_width < bar_width {
            crossterm::queue!(stdout, MoveRight((bar_width - progress_width) as u16))?;
        }

        write!(stdout, "]")?;
        stdout.flush()?;
        crossterm::queue!(stdout, SetAttribute(Attribute::Reset))?;

        Ok(())
    }

    pub fn set(&mut self, progress: f64) {
        self.progress = progress;
    }
}
