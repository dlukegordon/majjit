use clap::Parser;

const DEFAULT_REVSET: &str = "root() | remote_bookmarks() | ancestors(immutable_heads().., 50)";

#[derive(Parser, Debug)]
#[command(version, about = "Majjit: Magit for jj!")]
pub struct Args {
    /// Path to repository to operate on
    #[arg(short = 'R', long, default_value = ".")]
    pub repository: String,

    /// Which revisions to show
    #[arg(short = 'r', long, value_name = "REVSETS", default_value = DEFAULT_REVSET)]
    pub revisions: String,
}
