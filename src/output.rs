use std::cmp::Ordering;
use std::fmt::{Display, Write as _};
use std::io::{self, Write as _};
use std::sync::Mutex;

use crossterm::cursor::{self, MoveDown, MoveToColumn, MoveUp};
use crossterm::style::{style, Colorize, Styler};

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
    inner: Mutex<BlockInner>,
}

#[derive(Debug)]
struct BlockInner {
    row: u16,
}

#[derive(Copy, Clone, Debug)]
pub struct Line<'out, 'block> {
    block: &'block Block<'out>,
    row: u16,
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

    pub fn write_error(&self, err: &crate::Error) {
        self.write(|stdout| err.write(stdout)).ok();
    }

    pub fn write_block<'out, T, E>(&'out self, title: T, entries: E) -> crate::Result<Block<'out>>
    where
        T: Display,
        E: IntoIterator,
        E::Item: Display,
    {
        let mut stdout = self.stdout.lock();

        writeln!(stdout, "{}", style(&title).yellow().underlined())?;

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

        Ok(Block {
            stdout: &self.stdout,
            len,
            padding_cols: padding,
            remaining_cols: cols.saturating_sub(padding),
            inner: Mutex::new(BlockInner { row: len }),
        })
    }
}

impl<'out> Block<'out> {
    pub fn line<'block>(&'block self, row: u16) -> Line<'out, 'block>
    where
        'out: 'block,
    {
        Line {
            block: self,
            row,
            cols: self.remaining_cols,
        }
    }
}

impl<'out> Block<'out> {
    fn write<F>(&self, row: u16, write: F) -> crate::Result<()>
    where
        F: FnOnce(&mut io::StdoutLock<'out>) -> crate::Result<()>,
    {
        let mut stdout = self.stdout.lock();
        let mut inner = self.inner.lock().unwrap();

        inner.move_to_row(&mut stdout, row)?;
        inner.move_to_col(&mut stdout, self.padding_cols)?;
        write(&mut stdout)?;
        stdout.flush()?;

        Ok(())
    }
}

impl BlockInner {
    fn move_to_col(&mut self, stdout: &mut io::StdoutLock, col: u16) -> crate::Result<()> {
        crossterm::queue!(stdout, MoveToColumn(col))?;
        Ok(())
    }

    fn move_to_row(&mut self, stdout: &mut io::StdoutLock, row: u16) -> crate::Result<()> {
        match Ord::cmp(&self.row, &row) {
            Ordering::Greater => crossterm::queue!(stdout, MoveUp(self.row - row))?,
            Ordering::Equal => (),
            Ordering::Less => crossterm::queue!(stdout, MoveDown(row - self.row))?,
        }
        self.row = row;
        Ok(())
    }

    fn finish(&mut self, stdout: &mut io::StdoutLock, len: u16) -> crate::Result<()> {
        self.move_to_row(stdout, len)?;
        crossterm::queue!(stdout, MoveToColumn(0))?;
        stdout.flush()?;
        Ok(())
    }
}

impl<'out> Drop for Block<'out> {
    fn drop(&mut self) {
        let mut stdout = self.stdout.lock();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.finish(&mut stdout, self.len).ok();
    }
}

impl<'out, 'block> Line<'out, 'block> {
    pub fn write<F>(&self, write: F) -> crate::Result<()>
    where
        F: FnOnce(&mut io::StdoutLock<'out>) -> crate::Result<()>,
    {
        self.block.write(self.row, write)
    }

    pub fn write_error(&self, err: &crate::Error) {
        self.write(|stdout| err.write(stdout)).ok();
    }

    pub fn columns(&self) -> u16 {
        self.cols
    }
}
