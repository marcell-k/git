use crate::objects::{Kind, Object};
use anyhow::Context;
use std::fs;
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

struct DirEntry {
    entry: fs::DirEntry,
    meta: fs::Metadata,
    sort_key: Vec<u8>,
}

fn mode_for(meta: &fs::Metadata) -> &'static str {
    if meta.is_dir() {
        "40000"
    } else if meta.is_symlink() {
        "120000"
    } else if meta.permissions().mode() & 0o111 != 0 {
        "100755" // has at least one executable bit set
    } else {
        "100644"
    }
}

fn sort_key_for(name: &std::ffi::OsStr, is_dir: bool) -> Vec<u8> {
    let mut key = name.as_encoded_bytes().to_vec();
    if is_dir {
        key.push(b'/');
    }
    key
}

pub(crate) fn write_tree_for(path: &Path) -> anyhow::Result<Option<[u8; 20]>> {
    let dir = fs::read_dir(path).with_context(|| format!("open directory {}", path.display()))?;

    let mut entries = Vec::new();
    for entry in dir {
        let entry = entry.with_context(|| format!("bad directory entry in {}", path.display()))?;
        if entry.file_name() == ".git" {
            continue;
        }
        let meta = entry.metadata().context("metadata for directory entry")?;
        let sort_key = sort_key_for(&entry.file_name(), meta.is_dir());
        entries.push(DirEntry {
            entry,
            meta,
            sort_key,
        });
    }
    entries.sort_unstable_by(|a, b| a.sort_key.cmp(&b.sort_key));

    let mut tree_object = Vec::new();
    for DirEntry { entry, meta, .. } in entries {
        let path = entry.path();
        let hash = if meta.is_dir() {
            let Some(hash) = write_tree_for(&path)? else {
                continue;
            };
            hash
        } else if meta.is_symlink() {
            let target = fs::read_link(&path).context("read symlink target")?;
            let content = target.as_os_str().as_encoded_bytes();
            Object {
                kind: Kind::Blob,
                expected_size: content.len() as u64,
                reader: Cursor::new(content.to_vec()),
            }
            .write_to_objects()
            .context("write symlink blob")?
        } else {
            Object::blob_from_file(&path)
                .context("open blob input file")?
                .write_to_objects()
                .context("write blob object")?
        };

        tree_object.extend(mode_for(&meta).as_bytes());
        tree_object.push(b' ');
        tree_object.extend(entry.file_name().as_encoded_bytes());
        tree_object.push(0);
        tree_object.extend(hash);
    }

    if tree_object.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            Object {
                kind: Kind::Tree,
                expected_size: tree_object.len() as u64,
                reader: Cursor::new(tree_object),
            }
            .write_to_objects()
            .context("write tree object")?,
        ))
    }
}

pub(crate) fn invoke() -> anyhow::Result<()> {
    let Some(hash) = write_tree_for(Path::new(".")).context("construct root tree object")? else {
        anyhow::bail!("asked to make tree object for empty tree");
    };

    println!("{}", hex::encode(hash));

    Ok(())
}
