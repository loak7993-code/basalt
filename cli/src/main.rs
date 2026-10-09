/*
 * Basalt CLI
 * Copyright (C) 2026 Leonxlnx (loak7993-code)
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */



use basalt_cli::{accounts, install, launch, paths};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "basalt", version, about = "Basalt launcher for Linux")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Install a Minecraft version from the official manifest
    Install {
        /// Version id, e.g. 26.3 (default: latest release)
        version: Option<String>,
    },
    /// List installed versions
    List,
    /// Launch an installed version with an offline account
    Play {
        /// Version id (default: latest installed)
        version: Option<String>,
        /// Offline account name (default: the stored account)
        #[arg(long)]
        name: Option<String>,
        /// Server to join after launch
        #[arg(long)]
        server: Option<String>,
        /// Extra JVM arguments
        #[arg(long = "jvm")]
        jvm_args: Vec<String>,
        /// RAM for the game JVM in MB
        #[arg(long)]
        ram: Option<i64>,
    },
    /// Store the offline account used for launches
    Account {
        /// Offline account name
        name: String,
    },
    /// Show the paths Basalt uses
    Paths,
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Install { version } => install::run(version.as_deref()),
        Command::List => install::list(),
        Command::Play {
            version,
            name,
            server,
            jvm_args,
            ram,
        } => launch::run(version.as_deref(), name.as_deref(), server.as_deref(), &jvm_args, ram),
        Command::Account { name } => accounts::set(&name),
        Command::Paths => {
            println!("data:   {}", paths::data_dir().display());
            println!("config: {}", paths::config_dir().display());
            0
        }
    };
    std::process::exit(code);
}
