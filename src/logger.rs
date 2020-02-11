use std::io::{self, Write};

use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::cli;

pub fn init(args: &cli::Args) {
    log::set_boxed_logger(Box::new(Logger {
        stderr: StandardStream::stderr(args.color_choice(atty::Stream::Stderr)),
        stdout: StandardStream::stdout(ColorChoice::Never),
    }))
    .unwrap();
    log::set_max_level(args.logger.level_filter());
}

#[derive(Copy, Clone, Debug, StructOpt)]
#[structopt(no_version)]
pub struct Opts {
    #[structopt(
        long,
        help = "Enable debug logging",
        conflicts_with = "quiet",
        global = true
    )]
    debug: bool,
    #[structopt(
        long,
        help = "Enable trace logging",
        conflicts_with = "quiet",
        global = true
    )]
    trace: bool,
    #[structopt(long, short, help = "Disable all logging to stderr", global = true)]
    quiet: bool,
}

impl Opts {
    fn level_filter(&self) -> log::LevelFilter {
        if self.quiet {
            log::LevelFilter::Off
        } else if self.trace {
            log::LevelFilter::Trace
        } else if self.debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Warn
        }
    }
}

struct Logger {
    stderr: StandardStream,
    stdout: StandardStream,
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

        let _stdout_lock = self.stdout.lock();

        let mut writer = self.stderr.lock();
        let mut lines = msg.as_ref().lines();

        if let Some(first) = lines.next() {
            writer.set_color(ColorSpec::new().set_fg(Some(color)))?;
            write!(writer, "{:>pad$} ", prefix, pad = PAD)?;
            writer.reset()?;
            writeln!(writer, "{}", first)?;
        }
        for line in lines {
            writeln!(writer, "{:>pad$} {}", "", line, pad = PAD)?;
        }

        Ok(())
    }
}
