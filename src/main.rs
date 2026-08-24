use anyhow::{Context, Ok};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

pub(crate) mod commands;
pub(crate) mod objects;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    CatFile {
        #[arg(short = 'p')]
        pretty_print: bool,

        object_hash: String,
    },
    HashObject {
        #[arg(short = 'w')]
        write: bool,

        file: PathBuf,
    },
    LsTree {
        #[arg(long)]
        name_only: bool,
        tree_hash: String,
    },
    WriteTree,
    CommitTree {
        #[arg(short)]
        message: String,
        tree_hash: String,
        #[arg(short)]
        parent_hash: Option<String>,
    },
    Commit {
        #[arg(short)]
        message: String,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => commands::init::init_repo()?,
        Command::CatFile {
            pretty_print,
            object_hash,
        } => commands::cat_file::invoke(pretty_print, &object_hash)?,
        Command::HashObject { write, file } => commands::hash_object::invoke(write, &file)?,
        Command::LsTree {
            name_only,
            tree_hash,
        } => commands::ls_tree::invoke(name_only, &tree_hash)?,
        Command::WriteTree => commands::write_tree::invoke()?,
        Command::CommitTree {
            message,
            tree_hash,
            parent_hash,
        } => commands::commit_tree::invoke(message, tree_hash, parent_hash)?,
        Command::Commit { message } => {
            let head_ref = std::fs::read_to_string(".git/HEAD").context("read head")?;
            let Some(head_ref) = head_ref.strip_prefix("ref: ") else {
                anyhow::bail!("refuse to commit ontu detached HEAD");
            };
            let head_ref = head_ref.trim();
            let parent_hash = match std::fs::read_to_string(format!(".git/{head_ref}")) {
                Result::Ok(s) => Some(s.trim().to_string()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => {
                    return Err(e).with_context(|| format!("read ref HEAD target '{head_ref}'"));
                }
            };
            let Some(tree_hash) =
                commands::write_tree::write_tree_for(Path::new(".")).context("write tree")?
            else {
                eprintln!("not commiting empty tree");
                return Ok(());
            };
            let commit_hash = commands::commit_tree::write_commit(
                &message,
                &hex::encode(tree_hash),
                parent_hash.as_deref(),
            )?;
            let commit_hash = hex::encode(commit_hash);

            let ref_path = format!(".git/{head_ref}");
            let tmp_path = format!("{ref_path}.tmp");
            std::fs::write(&tmp_path, &commit_hash).context("write temp ref file")?;
            std::fs::rename(&tmp_path, &ref_path)
                .context("atomically update HEAD reference target")?;

            println!("HEAD is at {commit_hash}")
        }
    }

    Ok(())
}
