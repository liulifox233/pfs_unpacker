mod pack;
mod unpack;

use clap::{Parser, Subcommand};

use crate::unpack::unpack;

#[derive(Parser, Debug, Clone)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    /// Unpack an Artemis PFS archive
    Unpack { path: String },
    /// Pack a directory into an Artemis PFS archive
    Pack {
        #[clap(short, long)]
        input_dir: String,
        #[clap(short, long)]
        output_path: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Command::Unpack { path } => unpack(&path).unwrap(),
        Command::Pack {
            input_dir,
            output_path,
        } => pack::pack(&input_dir, &output_path).unwrap(),
    };
    Ok(())
}

const ARCHIVE_MAGIC: [u8; 2] = [0x70, 0x66];
const ARCHIVE_MAGIC_SIZE: usize = ARCHIVE_MAGIC.len();

#[repr(packed)]
struct ArtemisHeader {
    magic: [u8; ARCHIVE_MAGIC_SIZE],
    pack_version: u8,
    index_size: u32,
    file_count: u32,
}

struct ArtemisEntry {
    path: String,
    offset: u32,
    size: u32,
}
