mod cli;

fn main() {
    human_panic::setup_panic!();

    let args = cli::parse_args();
}
