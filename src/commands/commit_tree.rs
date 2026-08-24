use crate::objects::{Kind, Object};
use anyhow::Context;
use std::env;
use std::fmt::Write;
use std::io::Cursor;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn write_commit(
    message: &str,
    tree_hash: &str,
    parent_hash: Option<&str>,
) -> anyhow::Result<[u8; 20]> {
    let mut commit = String::new();
    writeln!(commit, "tree {tree_hash}")?;
    if let Some(parent_hash) = parent_hash {
        writeln!(commit, "parent {parent_hash}")?
    }
    let (name, email) =
        if let (Some(name), Some(email)) = (env::var_os("NAME"), env::var_os("EMAIL")) {
            let name = name
                .into_string()
                .map_err(|_| anyhow::anyhow!("$NAME is not set"))?;
            let email = email
                .into_string()
                .map_err(|_| anyhow::anyhow!("$EMAIL is not set"))?;

            (name, email)
        } else {
            (String::from("Author name"), String::from("email"))
        };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock before unix epoch")?
        .as_secs();

    writeln!(commit, "author {name} <{email}> {now} +0100")?;
    writeln!(commit, "committer {name} <{email}> {now} +0100")?;
    writeln!(commit)?;
    writeln!(commit, "{message}")?;

    Object {
        kind: Kind::Commit,
        expected_size: commit.len() as u64,
        reader: Cursor::new(commit),
    }
    .write_to_objects()
    .context("write commit object")
}

pub(crate) fn invoke(
    message: String,
    tree_hash: String,
    parent_hash: Option<String>,
) -> anyhow::Result<()> {
    let hash =
        write_commit(&message, &tree_hash, parent_hash.as_deref()).context("create commit")?;
    println!("{}", hex::encode(hash));
    Ok(())
}
