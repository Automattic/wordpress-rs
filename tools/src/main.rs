use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::xcframework::CreateXCFramework;

mod xcframework;

#[derive(Parser)]
struct Opts {
    #[clap(subcommand)]
    action: Actions,
}

#[derive(Debug, Subcommand)]
enum Actions {
    #[clap(name = "create-xcframework")]
    CreateXCFramework(CreateXCFramework),
}

trait Action {
    fn run(&self) -> Result<()>;
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    match opts.action {
        Actions::CreateXCFramework(action) => action.run(),
    }
}
