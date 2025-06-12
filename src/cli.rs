use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "Majjit: Magit for Jj!")]
pub struct Args {
    /// The path to look in for a jj repo
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Which revisions to show
    #[arg(
        short = 'r',
        long = "revisions",
        value_name = "REVSETS",
        default_value = "all()"
    )]
    revisions: String,
}
