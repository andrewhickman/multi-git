use std::io::Write;
use std::{fmt, io};

use crossterm::style::{Colorize, Styler};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Box<dyn std::error::Error>,
}

#[derive(Debug)]
struct Context {
    message: String,
    error: Error,
}

impl Error {
    pub fn write(&self, stdout: &mut io::StdoutLock) -> Result<()> {
        write!(stdout, "{}{}", "error: ".red().bold(), self)?;
        let mut err = self as &dyn std::error::Error;
        while let Some(source) = err.source() {
            write!(stdout, ": {}", source)?;
            err = source;
        }
        Ok(())
    }

    pub fn from_message(message: impl ToString) -> Self {
        Error {
            inner: message.to_string().into(),
        }
    }

    pub fn with_context(error: impl Into<Self>, message: impl ToString) -> Self {
        Self::from(Context {
            message: message.to_string(),
            error: error.into(),
        })
    }
}

impl From<git2::Error> for Error {
    fn from(err: git2::Error) -> Error {
        Error {
            inner: err.message().into(),
        }
    }
}

impl From<crossterm::ErrorKind> for Error {
    fn from(err: crossterm::ErrorKind) -> Error {
        Error { inner: err.into() }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error { inner: err.into() }
    }
}

impl From<fmt::Error> for Error {
    fn from(err: fmt::Error) -> Error {
        Error { inner: err.into() }
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Error {
        Error { inner: err.into() }
    }
}

impl From<Context> for Error {
    fn from(ctx: Context) -> Error {
        Error { inner: ctx.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for Context {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
