use clap::{Parser, Subcommand};

use crate::init::{Init, InitOptions};

mod init;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Raylib
}

fn main() {
    let args = Args::parse();
    let mut app = Init::new();

    let Some(command) = args.command else {
        app.init(None).unwrap();
        return;
   };

    match command {
        Command::Raylib => {
            app.init(Some(InitOptions::raylib(None))).unwrap();
        },
    }
}