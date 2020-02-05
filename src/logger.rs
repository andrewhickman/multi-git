use std::io::{self, Write};

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn init(options: &Options) {
    log::set_boxed_logger(Box::new(Logger {
        writer: StandardStream::stderr(options.color_choice()),
    }))
    .unwrap();
    log::set_max_level(options.level_filter());
}

pub struct Options {
    pub debug: bool,
    pub trace: bool,
    pub quiet: bool,
    pub color_choice: Option<ColorChoice>,
}

impl Options {
    fn color_choice(&self) -> ColorChoice {
        match self.color_choice {
            Some(ColorChoice::Auto) | None => {
                if atty::is(atty::Stream::Stderr) {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            }
            Some(color_choice) => color_choice,
        }
    }

    fn level_filter(&self) -> log::LevelFilter {
        if self.quiet {
            log::LevelFilter::Off
        } else if self.trace {
            log::LevelFilter::Trace
        } else if self.debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        }
    }
}

struct Logger {
    writer: StandardStream,
}

impl log::Log for Logger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(&record.metadata()) {
            self.write(record.level(), &record.args().to_string())
                .unwrap_or_else(|err| {
                    if err.kind() != io::ErrorKind::BrokenPipe {
                        panic!("error writing to stderr: {}", err);
                    }
                });
        }
    }

    fn flush(&self) {}
}

impl Logger {
    fn write(&self, lvl: log::Level, msg: impl AsRef<str>) -> io::Result<()> {
        const PAD: usize = 8;

        let (prefix, color) = match lvl {
            log::Level::Trace => ("trace:", Color::White),
            log::Level::Debug => ("debug:", Color::Cyan),
            log::Level::Info => ("info:", Color::Magenta),
            log::Level::Warn => ("warning:", Color::Yellow),
            log::Level::Error => ("error:", Color::Red),
        };

        let mut writer = self.writer.lock();
        let mut lines = msg.as_ref().lines();

        if let Some(first) = lines.next() {
            writer.set_color(ColorSpec::new().set_fg(Some(color)))?;
            write!(writer, "{:>pad$} ", prefix, pad = PAD)?;
            writer.reset()?;
            writeln!(writer, "{}", first)?;
        }
        for line in lines {
            writeln!(writer, "{:>pad$}  {}", "", line, pad = PAD)?;
        }

        Ok(())
    }
}
