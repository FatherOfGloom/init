use clap::{Args, Parser, Subcommand};

use crate::{init::{Init, InitOptions}};

mod init;
mod script_builder;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<ActionCommand>,
}

#[derive(Subcommand, Debug)]
enum ActionCommand {
    #[command(visible_aliases = ["rl"])]
    Raylib,
    #[command(visible_aliases = ["a"])]
    Add {
        #[command(subcommand)]
        target: TargetCommandArgs
    },
    #[command(visible_aliases = ["r", "rm"])]
    Remove {
        #[command(subcommand)]
        target: TargetCommandArgs
    },
    #[command(visible_aliases = ["l", "ls"])]
    List {
        #[command(subcommand)]
        target: TargetCommand
    },
    #[command(visible_aliases = ["res"])]
    Reset {
        #[command(subcommand)]
        target: TargetCommand
    }
}

#[derive(Subcommand, Debug)]
enum TargetCommandArgs {
    #[command(visible_aliases = ["f", "flag", "flags"])]
    Cflags(MultiArgs)
}

#[derive(Subcommand, Debug)]
enum TargetCommand {
    #[command(visible_aliases = ["f", "flag", "flags"])]
    Cflags
}

#[derive(Args, Debug)]
struct MultiArgs {
    args: String
}

fn main() {
    let cli = Cli::parse();
    let mut init = Init::new();

    let Some(command) = cli.command else {
        init.init(None).unwrap();
        return;
    };

    match command {
        ActionCommand::Raylib => {
            init.init(Some(InitOptions::raylib(None)))
        },
        ActionCommand::Add { target } => match target {
            TargetCommandArgs::Cflags(MultiArgs{ args }) => {
                init.add_cflags(&args)
            }
        }
        ActionCommand::Remove { target } => match target {
            TargetCommandArgs::Cflags(MultiArgs { args }) => {
                init.remove_cflags(&args)
            }
        }
        ActionCommand::Reset { target } => match target {
            TargetCommand::Cflags => init.reset_cflags()
        },
        ActionCommand::List { target } => match target {
            TargetCommand::Cflags => init.list_cflags()
        }
    }.unwrap();
}