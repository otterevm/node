use crate::cmd::max_tps::MaxTpsArgs;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "otter-bench", version, about = "OtterEVM benchmarking tool", long_about = None)]
pub struct TempoBench {
    #[command(subcommand)]
    pub cmd: TempoBenchSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum TempoBenchSubcommand {
    RunMaxTps(MaxTpsArgs),
}
