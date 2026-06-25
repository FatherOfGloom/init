use std::mem::ManuallyDrop;

use clap::{Args, Parser, Subcommand};

use crate::{init::{Init, InitOptions}};

mod init;
mod script_builder;
mod dependency;
mod common;

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
    Cflags(MultiArgs),
    #[command(visible_aliases = ["d", "dep"])]
    Deps(MultiArgs)
}

#[derive(Subcommand, Debug)]
enum TargetCommand {
    #[command(visible_aliases = ["f", "flag", "flags"])]
    Cflags,
    #[command(visible_aliases = ["d", "dep"])]
    Deps,
}

#[derive(Args, Debug)]
struct MultiArgs {
    args: String
}

fn main() { 
    let cli = ManuallyDrop::new(Cli::parse());
    let mut init = Init::new();

    let Some(ref command) = cli.command else {
        init.init(None).unwrap();
        return;
    };

    match command {
        ActionCommand::Raylib => {
            init.init(Some(InitOptions::raylib(None)))
        },
        ActionCommand::Add { target } => match target {
            TargetCommandArgs::Cflags(MultiArgs{ args }) => init.add_cflags(&args),
            TargetCommandArgs::Deps(MultiArgs { args }) => init.add_dependencies(&args), 
        }
        ActionCommand::Remove { target } => match target {
            TargetCommandArgs::Cflags(MultiArgs { args }) => init.remove_cflags(&args),
            TargetCommandArgs::Deps(MultiArgs { args }) => init.remove_dependencies_by_names(&args),
        }
        ActionCommand::Reset { target } => match target {
            TargetCommand::Cflags => init.reset_cflags(),
            TargetCommand::Deps => init.reset_dependencies(),
        },
        ActionCommand::List { target } => match target {
            TargetCommand::Cflags => init.list_cflags(),
            TargetCommand::Deps => init.list_dependencies(),
        }
    }.unwrap();
}