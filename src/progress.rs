use crate::output;

use std::io::Write;

use crossterm::cursor::MoveRight;
use crossterm::style::{Attribute, PrintStyledContent, SetAttribute, Styler};
use crossterm::terminal::{Clear, ClearType};

#[derive(Clone, Debug)]
pub struct ProgressBar<'out, 'block> {
    line: output::Line<'out, 'block>,
    status_cols: u16,
    bar_cols: u16,
}

impl<'out, 'block> ProgressBar<'out, 'block> {
    pub fn new(line: output::Line<'out, 'block>, status_cols: u16) -> Self {
        ProgressBar {
            line,
            status_cols,
            bar_cols: line.columns().saturating_sub(status_cols + 2),
        }
    }

    pub fn begin(&self) -> crate::Result<()> {
        self.line.write(|stdout| {
            crossterm::queue!(
                stdout,
                MoveRight(self.status_cols),
                PrintStyledContent("[".dim()),
                MoveRight(self.bar_cols),
                PrintStyledContent("]".dim())
            )?;
            Ok(())
        })
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
            crossterm::queue!(stdout, SetAttribute(Attribute::NoBold))?;
            Ok(())
        })
    }

    fn finish(&self) -> crate::Result<()> {
        self.line.write(|stdout| {
            crossterm::queue!(
                stdout,
                MoveRight(self.status_cols),
                Clear(ClearType::UntilNewLine)
            )?;
            Ok(())
        })
    }
}

impl<'out, 'block> Drop for ProgressBar<'out, 'block> {
    fn drop(&mut self) {
        self.finish().ok();
    }
}
