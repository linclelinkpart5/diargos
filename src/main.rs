use std::cmp::Ordering;
use std::path::Path;
use std::path::PathBuf;

use cursive::Cursive;
use cursive::traits::Nameable;
use cursive::views::Dialog;
use cursive_tree_view::Placement;
use cursive_tree_view::TreeView;

#[derive(Debug)]
struct TreeEntry {
    name: String,
    dir: Option<PathBuf>,
}

impl std::fmt::Display for TreeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn collect_entries(dir: &Path, entries: &mut Vec<TreeEntry>) -> std::io::Result<()> {
    let metadata = std::fs::metadata(&dir)?;

    if metadata.is_dir() {
        for entry_res in std::fs::read_dir(dir)? {
            let entry = entry_res?;

            let sub_name = entry.file_name();
            let sub_path = dir.join(&sub_name);

            let sub_metadata = std::fs::metadata(&sub_path)?;

            let sub_dir =
                if sub_metadata.is_dir() { Some(sub_path) }
                else if sub_metadata.is_file() { None }
                else { continue; }
            ;

            entries.push(
                TreeEntry {
                    name: sub_name.to_string_lossy().into_owned(),
                    dir: sub_dir,
                }
            );
        }
    }

    Ok(())
}

fn expand_tree(tree: &mut TreeView<TreeEntry>, parent_row: usize, dir: &Path) {
    let mut entries = Vec::new();

    collect_entries(dir, &mut entries).ok();

    entries.sort_by(|a, b| {
        // Directories always go at the top.
        match (a.dir.is_some(), b.dir.is_some()) {
            (true, true) | (false, false) => a.name.cmp(&b.name),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
        }
    });

    for i in entries {
        if i.dir.is_some() {
            tree.insert_container_item(i, Placement::LastChild, parent_row);
        }
        else {
            tree.insert_item(i, Placement::LastChild, parent_row);
        }
    }
}

fn main() {
    // Create TreeView with initial working directory
    let mut tree = TreeView::<TreeEntry>::new();
    let path = std::env::current_dir().expect("Working directory missing.");

    tree.insert_item(
        TreeEntry {
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            dir: Some(path.clone()),
        },
        Placement::After,
        0,
    );

    expand_tree(&mut tree, 0, &path);

    // Lazily insert directory listings for sub nodes
    tree.set_on_collapse(|siv: &mut Cursive, row, is_collapsed, children| {
        if !is_collapsed && children == 0 {
            siv.call_on_name("tree", move |tree: &mut TreeView<TreeEntry>| {
                if let Some(dir) = tree.borrow_item(row).unwrap().dir.clone() {
                    expand_tree(tree, row, &dir);
                }
            });
        }
    });

    // Setup Cursive
    let mut siv = Cursive::default();
    siv.add_layer(Dialog::around(tree.with_name("tree")).title("File View"));

    siv.run();
}
