
use std::cmp::Ordering;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::io::Result as IoResult;
use std::path::PathBuf;

use cursive_tree_view::TreeView;
use cursive_tree_view::Placement;

#[derive(Debug)]
struct BrowserEntry {
    name: String,
    dir: Option<PathBuf>,
}

impl Display for BrowserEntry {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.name)
    }
}

fn collect_entries(dir: &PathBuf, entries: &mut Vec<BrowserEntry>) -> IoResult<()> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                entries.push(BrowserEntry {
                    name: entry
                        .file_name()
                        .into_string()
                        .unwrap_or_else(|_| "".to_string()),
                    dir: Some(path),
                });
            } else if path.is_file() {
                entries.push(BrowserEntry {
                    name: entry
                        .file_name()
                        .into_string()
                        .unwrap_or_else(|_| "".to_string()),
                    dir: None,
                });
            }
        }
    }
    Ok(())
}

pub struct FileBrowserView {
    tree_view: TreeView<BrowserEntry>,
}

impl FileBrowserView {
    fn expand_tree(&mut self, parent_row: usize, dir: &PathBuf) {
        let mut entries = Vec::new();
        collect_entries(dir, &mut entries).ok();

        entries.sort_by(|a, b| {
            match (a.dir.is_some(), b.dir.is_some()) {
                (true, true) | (false, false) => a.name.cmp(&b.name),
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
            }
        });

        for entry in entries {
            if entry.dir.is_some() {
                self.tree_view.insert_container_item(
                    entry,
                    Placement::LastChild,
                    parent_row,
                );
            } else {
                self.tree_view.insert_item(
                    entry,
                    Placement::LastChild,
                    parent_row,
                );
            }
        }
    }
}
