mod cli;
mod logger;

fn main() {
    human_panic::setup_panic!();

    let args = cli::parse_args();
    logger::init(&args.logger_options());
    log::trace!("{:?}", args);
}
