use std::fs;

use anyhow::Error;

pub(crate) fn init_repo() -> anyhow::Result<(), Error> {
    fs::create_dir(".git").unwrap();
    fs::create_dir(".git/objects").unwrap();
    fs::create_dir(".git/refs").unwrap();
    fs::create_dir(".git/refs/heads").unwrap();
    fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
    println!("Initialized git directory");
    Ok(())
}
