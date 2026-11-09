use clap::Parser;

#[derive(Parser)]
#[command(name = "spine-space-mesh", version, about = "spine-space-mesh — auto-scaffold")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Run { task: String },
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run { task } => println!("spine-space-mesh: {task}"),
        Commands::Status => println!("spine-space-mesh ok (port 8080)"),
    }
}
