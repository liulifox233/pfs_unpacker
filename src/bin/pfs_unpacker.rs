use clap::{Parser, Subcommand};
use pfs_unpacker::{
    pack::pack,
    unpack::{get_info, unpack},
};

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
        } => pack(&input_dir, &output_path, version),
    };
    Ok(())
}

#[cfg(test)]
mod test {
    use assert_cmd::Command;
    use std::fs;

    #[test]
    fn test_pack_v6() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;

        cmd.arg("pack")
            .arg("tests/test_pack")
            .arg("/tmp/test_pack_v6.pfs");
        cmd.assert().success();

        let expected = fs::read("tests/test_pack_v6.pfs")?;
        let packed = fs::read("/tmp/test_pack_v6.pfs")?;

        assert_eq!(expected, packed);

        fs::remove_file("/tmp/test_pack_v6.pfs")?;

        Ok(())
    }

    #[test]
    fn test_unpack_v6() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;

        cmd.arg("unpack")
            .arg("tests/test_pack_v6.pfs")
            .arg("/tmp/test_pack_v6");
        cmd.assert().success();

        let expected = fs::read_dir("tests/test_pack")?;
        let unpacked = fs::read_dir("/tmp/test_pack_v6")?;

        for (expected, unpacked) in expected.zip(unpacked) {
            let expected = expected?;
            let unpacked = unpacked?;

            assert_eq!(expected.file_name(), unpacked.file_name());
        }

        fs::remove_dir_all("/tmp/test_pack_v6")?;

        Ok(())
    }

    #[test]
    fn test_pack_v8() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;

        cmd.arg("pack")
            .arg("tests/test_pack")
            .arg("/tmp/test_pack_v8.pfs")
            .arg("--version")
            .arg("8");
        cmd.assert().success();

        let expected = fs::read("tests/test_pack_v8.pfs")?;
        let packed = fs::read("/tmp/test_pack_v8.pfs")?;

        assert_eq!(expected, packed);

        fs::remove_file("/tmp/test_pack_v8.pfs")?;

        Ok(())
    }

    #[test]
    fn test_unpack_v8() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;

        cmd.arg("unpack")
            .arg("tests/test_pack_v8.pfs")
            .arg("/tmp/test_pack_v8");
        cmd.assert().success();

        let expected = fs::read_dir("tests/test_pack")?;
        let unpacked = fs::read_dir("/tmp/test_pack_v8")?;

        for (expected, unpacked) in expected.zip(unpacked) {
            let expected = expected?;
            let unpacked = unpacked?;

            assert_eq!(expected.file_name(), unpacked.file_name());
        }

        fs::remove_dir_all("/tmp/test_pack_v8")?;

        Ok(())
    }
}
