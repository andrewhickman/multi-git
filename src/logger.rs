use std::{
    env,
    fmt::Arguments,
    io::{self, LineWriter, Write},
    path::Path,
    sync::Mutex,
    time::Duration,
};

use chrono::{DateTime, Utc};
use fs_err::File;
use log::Log;
use serde::Serialize;

pub fn init() -> io::Result<()> {
    let logger = Logger::new()?;

    log::set_max_level(log::LevelFilter::Trace);
    log::set_boxed_logger(Box::new(logger)).unwrap();

    Ok(())
}

struct Logger {
    file: Mutex<LineWriter<File>>,
}

#[derive(Serialize)]
struct JsonRecord<'a> {
    timestamp: DateTime<Utc>,
    level: log::Level,
    target: &'a str,
    message: &'a Arguments<'a>,
}

impl Logger {
    fn new() -> io::Result<Self> {
        let log_dir = dirs::data_dir()
            .unwrap_or_else(env::temp_dir)
            .join(env!("CARGO_PKG_NAME"))
            .join("logs");

        fs_err::create_dir_all(&log_dir)?;
        clean_log_dir(&log_dir)?;

        Ok(Logger {
            file: Mutex::new(LineWriter::new(File::create(log_dir.join(format!(
                "{}-{}.log",
                env!("CARGO_PKG_NAME"),
                Utc::now().format("%Y%m%d-%H%M%S")
            )))?)),
        })
    }
}

fn clean_log_dir(log_dir: &Path) -> io::Result<()> {
    for entry in fs_err::read_dir(log_dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;

        if meta.is_file()
            && matches!(meta.modified()?.elapsed(), Ok(elapsed) if elapsed > Duration::from_secs(604800))
        {
            fs_err::remove_file(entry.path())?;
        }
    }

    Ok(())
}

impl Log for Logger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let mut file = self.file.lock().unwrap();
            serde_json::to_writer(
                &mut *file,
                &JsonRecord {
                    timestamp: Utc::now(),
                    level: record.metadata().level(),
                    target: record.target(),
                    message: record.args(),
                },
            )
            .ok();
            writeln!(&mut *file).ok();
        }
    }

    fn flush(&self) {
        self.file.lock().unwrap().flush().ok();
    }
}
