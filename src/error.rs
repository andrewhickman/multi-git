use std::io::Write;
use std::{fmt, io};

use crossterm::style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor};
use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

#[derive(Debug)]
struct Context {
    message: String,
    error: Error,
}

impl Error {
    pub fn write(&self, stdout: &mut io::StdoutLock) -> crossterm::Result<()> {
        crossterm::queue!(
            stdout,
            SetForegroundColor(Color::Red),
            SetAttribute(Attribute::Bold)
        )?;
        write!(stdout, "error: ")?;
        stdout.flush()?;
        crossterm::queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;

        write!(stdout, "{}", self)?;
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

    pub fn context(self, message: impl ToString) -> Self {
        Error::with_context(self, message)
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

impl From<toml_edit::TomlError> for Error {
    fn from(err: toml_edit::TomlError) -> Error {
        Error { inner: err.into() }
    }
}

impl From<Context> for Error {
    fn from(ctx: Context) -> Error {
        Error { inner: ctx.into() }
    }
}

impl From<serde_json::Error> for Error {
    fn from(ctx: serde_json::Error) -> Error {
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

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct JsonError {
            message: String,
            source: Option<Box<JsonError>>,
        }

        fn to_json_error(err: &dyn std::error::Error) -> JsonError {
            JsonError {
                message: err.to_string(),
                source: err.source().map(to_json_error).map(Box::new),
            }
        }

        to_json_error(self).serialize(serializer)
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

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Error::from_message(msg)
    }
}
