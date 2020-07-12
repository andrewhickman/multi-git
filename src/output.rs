use std::fmt::{Display, Write as _};
use std::io::{self, Write as _};

use crossterm::cursor::{self, MoveTo, MoveToColumn};
use crossterm::style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor};

use crate::progress::ProgressBar;

#[derive(Debug)]
pub struct Output {
    stdout: io::Stdout,
}

#[derive(Debug)]
pub struct Block<'out> {
    stdout: &'out io::Stdout,
    len: u16,
    padding_cols: u16,
    remaining_cols: u16,
    row: i16,
}

#[derive(Copy, Clone, Debug)]
pub struct Line<'out, 'block> {
    block: &'block Block<'out>,
    index: u16,
    cols: u16,
}

impl Output {
    pub fn new() -> Self {
        Output {
            stdout: io::stdout(),
        }
    }

    pub fn write<'out, F>(&'out self, write: F) -> crate::Result<()>
    where
        F: FnOnce(&mut io::StdoutLock) -> crate::Result<()>,
    {
        write(&mut self.stdout.lock())
    }

    pub fn writeln<'out, F>(&'out self, write: F) -> crate::Result<()>
    where
        F: FnOnce(&mut io::StdoutLock) -> crate::Result<()>,
    {
        let mut stdout = self.stdout.lock();
        write(&mut stdout)?;
        writeln!(stdout)?;
        Ok(())
    }

    pub fn write_error(&self, err: &crate::Error) {
        self.write(|stdout| err.write(stdout)).ok();
    }

    pub fn writeln_error(&self, err: &crate::Error) {
        self.writeln(|stdout| err.write(stdout)).ok();
    }

    pub fn write_block<'out, T, E>(&'out self, title: T, entries: E) -> crate::Result<Block<'out>>
    where
        T: Display,
        E: IntoIterator,
        E::Item: Display,
    {
        let mut stdout = self.stdout.lock();

        crossterm::queue!(stdout, cursor::Hide, cursor::DisableBlinking)?;

        crossterm::queue!(
            stdout,
            SetForegroundColor(Color::Yellow),
            SetAttribute(Attribute::Underlined)
        )?;
        writeln!(stdout, "{}", &title)?;
        stdout.flush()?;
        crossterm::queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;

        let (cols, _) = crossterm::terminal::size()?;

        let mut len = 0;
        let mut padding = cols / 2;
        let mut buf = String::with_capacity(cols as usize);
        for entry in entries {
            write!(buf, "{} ", entry)?;
            padding = padding.max(buf.len() as u16);
            writeln!(stdout, "{}", buf)?;
            buf.clear();
            len += 1;
        }

        let (_, row) = crossterm::cursor::position()?;

        Ok(Block {
            stdout: &self.stdout,
            len,
            padding_cols: padding,
            remaining_cols: cols.saturating_sub(padding),
            row: row as i16 - len as i16,
        })
    }
}

impl<'out> Block<'out> {
    pub fn line<'block>(&'block self, index: u16) -> Line<'out, 'block>
    where
        'out: 'block,
    {
        Line {
            block: self,
            index,
            cols: self.remaining_cols,
        }
    }
}

impl<'out> Block<'out> {
    fn write<F>(&self, index: u16, write: F) -> crate::Result<()>
    where
        F: FnOnce(&mut io::StdoutLock<'out>) -> crate::Result<()>,
    {
        let mut stdout = self.stdout.lock();

        if self.move_to_row(&mut stdout, index)? {
            self.move_to_col(&mut stdout, self.padding_cols)?;
            write(&mut stdout)?;
            stdout.flush()?;
        }

        Ok(())
    }

    fn move_to_col(&self, stdout: &mut io::StdoutLock, col: u16) -> crate::Result<()> {
        crossterm::queue!(stdout, MoveToColumn(col))?;
        Ok(())
    }

    fn move_to_row(&self, stdout: &mut io::StdoutLock, index: u16) -> crate::Result<bool> {
        let row = self.row + index as i16;
        if row >= 0 {
            crossterm::queue!(stdout, MoveTo(0, row as u16))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn finish(&self, stdout: &mut io::StdoutLock, len: u16) -> crate::Result<()> {
        self.move_to_row(stdout, len)?;
        crossterm::queue!(
            stdout,
            MoveToColumn(0),
            cursor::Show,
            cursor::EnableBlinking
        )?;
        stdout.flush()?;
        Ok(())
    }
}

impl<'out> Drop for Block<'out> {
    fn drop(&mut self) {
        let mut stdout = self.stdout.lock();
        self.finish(&mut stdout, self.len).ok();
    }
}

impl<'out, 'block> Line<'out, 'block> {
    pub fn write<F>(&self, write: F) -> crate::Result<()>
    where
        F: FnOnce(&mut io::StdoutLock<'out>) -> crate::Result<()>,
    {
        self.block.write(self.index, write)
    }

    pub fn write_error(&self, err: &crate::Error) {
        self.write(|stdout| err.write(stdout)).ok();
    }

    pub fn write_progress<F>(
        &self,
        status_cols: u16,
        write_status: F,
    ) -> crate::Result<ProgressBar<'out, 'block>>
    where
        F: FnOnce(&mut io::StdoutLock<'out>) -> crate::Result<()>,
    {
        let bar = ProgressBar::new(*self, status_cols);
        self.write(write_status)?;
        bar.begin()?;
        Ok(bar)
    }

    pub fn columns(&self) -> u16 {
        self.cols
    }
}
