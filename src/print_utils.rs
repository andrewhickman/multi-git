use std::io;

use std::path::Path;

use termcolor::{Color, ColorSpec, WriteColor};

pub fn print_dir(stdout: &mut impl WriteColor, path: &Path) -> io::Result<()> {
    stdout.set_color(
        &ColorSpec::new()
            .set_fg(Some(Color::Yellow))
            .set_bold(true)
            .set_underline(true),
    )?;
    write!(stdout, "{}", path.display())?;
    stdout.reset()?;
    writeln!(stdout)
}

pub fn handle_print_error(err: io::Error) {
    log::error!("failed to write to stdout\ncaused by: {}", err);
}
