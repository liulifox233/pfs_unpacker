mod pack;
mod unpack;

use clap::{Parser, Subcommand};
use unpack::get_info;

use crate::unpack::unpack;

#[derive(Parser, Debug, Clone)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    /// Display information about an Artemis PFS archive
    Info {
        /// Path to the archive
        path: String,
        /// Display detailed information about the archive
        #[clap(short, long)]
        verbose: bool,
    },
    /// Unpack an Artemis PFS archive
    Unpack {
        /// Path to the archive
        path: String,
        /// Output directory
        output_dir: Option<String>,
    },
    /// Pack a directory into an Artemis PFS archive
    Pack {
        /// Input directory
        input_dir: String,
        /// Output path
        output_path: String,
        /// Pack version
        #[clap(short, long)]
        version: Option<u8>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Command::Info { path, verbose } => get_info(path, verbose).unwrap(),
        Command::Unpack { path, output_dir } => unpack(path, output_dir).unwrap(),
        Command::Pack {
            input_dir,
            output_path,
            version,
        } => pack::pack(&input_dir, &output_path, version).unwrap(),
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
