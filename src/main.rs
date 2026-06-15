use clap::{Args, Parser, Subcommand};

use crate::{init::{Init, InitOptions}};

mod init;
mod script_builder;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Raylib,
    Add {
        #[command(subcommand)]
        action: AddCommand
    },
    Remove {
        #[command(subcommand)]
        action: RemoveCommand
    },
    Reset {
        #[command(subcommand)]
        action: ResetCommand
    },
    List {
        #[command(subcommand)]
        action: ListCommand
    }
}

#[derive(Subcommand, Debug)]
enum AddCommand {
    Cflags(MultiArgs)
}

#[derive(Subcommand, Debug)]
enum RemoveCommand {
    Cflags(MultiArgs)
}

#[derive(Subcommand, Debug)]
enum ResetCommand {
    Cflags
}

#[derive(Subcommand, Debug)]
enum ListCommand {
    Cflags
}

#[derive(Args, Debug)]
struct MultiArgs {
    args: String
}

fn main() {
    let cli = Cli::parse();
    let mut app = Init::new();

    let Some(command) = cli.command else {
        app.init(None).unwrap();
        return;
   };

    match command {
        Command::Raylib => {
            app.init(Some(InitOptions::raylib(None))).unwrap();
        },
        Command::Add { action } => match action {
            AddCommand::Cflags(MultiArgs{ args }) => {
                app.add_cflags(&args).unwrap();
            }
        }
        Command::Remove { action } => match action {
            RemoveCommand::Cflags(MultiArgs { args }) => {
                app.remove_cflags(&args).unwrap();
            }
        }
        Command::Reset { action } => match action {
            ResetCommand::Cflags => app.reset_cflags().unwrap()
        },
        Command::List { action } => match action {
            ListCommand::Cflags => app.list_cflags().unwrap()
        }
    }
}