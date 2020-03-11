use crate::output;

use std::io::Write;

use crossterm::cursor::MoveRight;
use crossterm::style::{Attribute, SetAttribute};
use crossterm::terminal::{Clear, ClearType};

#[derive(Clone, Debug)]
pub struct ProgressBar<'out, 'block> {
    line: output::Line<'out, 'block>,
    status_cols: u16,
    bar_cols: u16,
    finished: bool,
}

impl<'out, 'block> ProgressBar<'out, 'block> {
    pub fn new(line: output::Line<'out, 'block>, status_cols: u16) -> Self {
        ProgressBar {
            line,
            status_cols,
            bar_cols: line.columns().saturating_sub(status_cols + 2),
            finished: false,
        }
    }

    pub fn begin(&self) -> crate::Result<()> {
        self.line.write(|stdout| {
            crossterm::queue!(
                stdout,
                MoveRight(self.status_cols),
                SetAttribute(Attribute::Dim),
            )?;
            write!(stdout, "[")?;
            stdout.flush()?;
            crossterm::queue!(stdout, MoveRight(self.bar_cols))?;
            write!(stdout, "]")?;
            stdout.flush()?;
            crossterm::queue!(stdout, SetAttribute(Attribute::Reset))?;
            Ok(())
        })?;
        self.set(0.0)
    }

    pub fn set(&self, progress: f64) -> crate::Result<()> {
        let length = (self.bar_cols as f64 * progress) as usize;
        self.line.write(|stdout| {
            crossterm::queue!(
                stdout,
                MoveRight(self.status_cols + 1),
                SetAttribute(Attribute::Bold),
            )?;
            write!(stdout, "{:=>length$}", ">", length = length)?;
            crossterm::queue!(stdout, SetAttribute(Attribute::Reset))?;
            Ok(())
        })
    }

    pub fn finish(&mut self) -> crate::Result<output::Line<'out, 'block>> {
        self.line.write(|stdout| {
            crossterm::queue!(
                stdout,
                MoveRight(self.status_cols),
                Clear(ClearType::UntilNewLine)
            )?;
            Ok(())
        })?;
        self.finished = true;
        Ok(self.line)
    }
}

impl<'out, 'block> Drop for ProgressBar<'out, 'block> {
    fn drop(&mut self) {
        if !self.finished {
            self.finish().ok();
        }
    }
}
