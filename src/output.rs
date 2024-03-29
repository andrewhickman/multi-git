use std::cmp;
use std::fmt::Display;
use std::io::{self, Write as _};
use std::ops::Range;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crossterm::{
    cursor::{self, MoveToColumn, MoveUp},
    style::{SetAttribute, SetForegroundColor},
};
use crossterm::{
    style::{Attribute, Color, ResetColor},
    terminal,
};

pub struct Output {
    stdout: io::Stdout,
    json: bool,
}

pub struct Block<'out> {
    output: &'out Output,
    inner: Mutex<BlockInner<'out>>,
}

struct BlockInner<'out> {
    rows: usize,
    range: Range<usize>,
    entries: Vec<BlockEntry<'out>>,
}

struct BlockEntry<'out> {
    content: Arc<dyn LineContent + 'out>,
    finished: bool,
}

/// A single line of output
pub trait LineContent: Send + Sync {
    fn write(&self, stdout: &mut io::StdoutLock) -> crossterm::Result<()>;
    fn write_json(&self, stdout: &mut io::StdoutLock) -> serde_json::Result<()>;
}

pub struct Line<'out, 'block, C> {
    block: &'block Block<'out>,
    index: usize,
    content: Arc<C>,
}

impl Output {
    pub fn new(json: bool) -> Self {
        Output {
            stdout: io::stdout(),
            json,
        }
    }

    fn writeln<F>(&self, write: F) -> crate::Result<()>
    where
        F: FnOnce(&mut io::StdoutLock) -> crossterm::Result<()>,
    {
        let mut stdout = self.stdout.lock();
        write(&mut stdout)?;
        writeln!(stdout)?;
        Ok(())
    }

    fn writeln_json(&self, msg: &impl Serialize) -> io::Result<()> {
        let mut stdout = self.stdout.lock();
        serde_json::to_writer(&mut stdout, msg)?;
        writeln!(stdout)?;
        Ok(())
    }

    pub fn writeln_message(&self, msg: impl Display) {
        #[derive(Serialize)]
        struct JsonMessage {
            kind: &'static str,
            message: String,
        }

        if self.json {
            self.writeln_json(&JsonMessage {
                kind: "message",
                message: msg.to_string(),
            })
            .ok();
        } else {
            self.writeln(|stdout| {
                write!(stdout, "{}", msg)?;
                Ok(())
            })
            .ok();
        }
    }

    pub fn writeln_warning(&self, msg: impl Display) {
        self.writeln(|stdout| {
            crossterm::queue!(
                stdout,
                SetForegroundColor(Color::Yellow),
                SetAttribute(Attribute::Bold)
            )?;
            write!(stdout, "warning: ")?;
            stdout.flush()?;
            crossterm::queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;

            write!(stdout, "{}", msg)?;
            Ok(())
        })
        .ok();
    }

    pub fn writeln_error(&self, err: &crate::Error) {
        self.writeln(|stdout| err.write(stdout)).ok();
    }

    pub fn block(&self) -> crate::Result<Block<'_>> {
        if !self.json {
            terminal::enable_raw_mode()?;
            crossterm::queue!(self.stdout.lock(), cursor::Hide, cursor::DisableBlinking)?;
        }

        let (_, rows) = terminal::size()?;

        Ok(Block {
            output: self,
            inner: Mutex::new(BlockInner {
                rows: rows as usize,
                entries: vec![],
                range: 0..0,
            }),
        })
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        self.stdout.flush().ok();
    }
}

impl<'out> Block<'out> {
    pub fn add_line<'block, C>(&'block self, content: C) -> Line<'out, 'block, C>
    where
        C: LineContent + 'out,
    {
        let content = Arc::new(content);
        let index = self.inner.lock().unwrap().add_line(content.clone());

        Line {
            index,
            content,
            block: self,
        }
    }

    pub fn add_finished_line<C>(&self, content: C)
    where
        C: LineContent + 'out,
    {
        self.add_line(content).finish();
    }

    pub fn add_error_line(&self, error: crate::Error) {
        self.add_finished_line(ErrorLineContent { error })
    }

    pub fn update_all(&self) -> crossterm::Result<()> {
        if !self.output.json {
            let mut inner = self.inner.lock().unwrap();
            let mut stdout = self.output.stdout.lock();

            inner.write_all(&mut stdout)?;
            inner.reset_cursor(&mut stdout)?;
        }

        Ok(())
    }

    fn update(&self, index: usize) -> crossterm::Result<()> {
        if !self.output.json {
            if let Ok(mut inner) = self.inner.try_lock() {
                let mut stdout = self.output.stdout.lock();

                inner.update(&mut stdout, index)?;
            }
        }

        Ok(())
    }

    fn finish(&self, index: usize) -> crossterm::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let mut stdout = self.output.stdout.lock();

        if self.output.json {
            inner.finish_json(&mut stdout, index)?;
        } else {
            inner.finish(&mut stdout, index)?;
        }

        Ok(())
    }
}

impl<'out> BlockInner<'out> {
    fn add_line(&mut self, content: Arc<dyn LineContent + 'out>) -> usize {
        let index = self.entries.len();
        self.entries.push(BlockEntry {
            content,
            finished: false,
        });

        if (self.range.len() + 1) < self.rows {
            self.range.end += 1;
        }

        index
    }

    fn update(&mut self, stdout: &mut io::StdoutLock, index: usize) -> crossterm::Result<()> {
        if self.range.contains(&index) {
            self.write_all(stdout)?;
            crossterm::queue!(stdout, MoveUp(self.range.len() as u16))?;
        }
        Ok(())
    }

    fn finish(&mut self, stdout: &mut io::StdoutLock, index: usize) -> crossterm::Result<()> {
        self.entries[index].finished = true;

        let shift = if index == self.range.start {
            self.entries[index..]
                .iter()
                .take_while(|entry| entry.finished)
                .count()
        } else {
            0
        };

        self.range.end = cmp::min(self.range.end + shift, self.entries.len());
        self.write_all(stdout)?;
        self.range.start += shift;

        self.reset_cursor(stdout)?;

        Ok(())
    }

    fn finish_json(&mut self, stdout: &mut io::StdoutLock, index: usize) -> io::Result<()> {
        self.entries[index].finished = true;

        for entry in self.entries[index..]
            .iter()
            .take_while(|entry| entry.finished)
        {
            entry.content.write_json(stdout)?;
            writeln!(stdout)?;
        }
        Ok(())
    }

    fn write_all(&mut self, stdout: &mut io::StdoutLock) -> crossterm::Result<()> {
        for index in self.range.clone() {
            self.entries[index].content.write(stdout)?;
            writeln!(stdout)?;
        }

        Ok(())
    }

    fn reset_cursor(&mut self, stdout: &mut io::StdoutLock) -> crossterm::Result<()> {
        if !self.range.is_empty() {
            crossterm::queue!(stdout, MoveUp(self.range.len() as u16))?;
        }
        Ok(())
    }
}

impl<'out> Drop for Block<'out> {
    fn drop(&mut self) {
        if !self.output.json {
            let mut inner = self.inner.lock().unwrap();
            let mut stdout = self.output.stdout.lock();

            inner.write_all(&mut stdout).ok();

            crossterm::queue!(
                &mut stdout,
                MoveToColumn(0),
                cursor::Show,
                cursor::EnableBlinking
            )
            .ok();
            terminal::disable_raw_mode().ok();
        }
    }
}

impl<'out, 'block, C> Line<'out, 'block, C> {
    pub fn content(&self) -> &C {
        &self.content
    }

    pub fn update(&self) {
        self.block.update(self.index).ok();
    }

    pub fn finish(&self) {
        self.block.finish(self.index).ok();
    }
}

struct ErrorLineContent {
    error: crate::Error,
}

impl LineContent for ErrorLineContent {
    fn write(&self, stdout: &mut io::StdoutLock) -> crossterm::Result<()> {
        self.error.write(stdout)
    }

    fn write_json(&self, stdout: &mut io::StdoutLock) -> serde_json::Result<()> {
        #[derive(Serialize)]
        struct JsonError<'a> {
            kind: &'static str,
            #[serde(flatten)]
            error: &'a crate::Error,
        }

        serde_json::to_writer(
            stdout,
            &JsonError {
                kind: "error",
                error: &self.error,
            },
        )
    }
}
