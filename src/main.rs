
mod config;
mod consts;
mod cursor;
mod data;
mod model;
mod util;
mod views;

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use indexmap::indexmap;
use str_macro::str;

use clap::Clap;
use cursive::Cursive;
use cursive::CursiveExt;
use cursive::views::Dialog;
use metaflac::Tag;
use metaflac::Block;

use crate::config::Config;
use crate::data::Data;
use crate::data::Record;
use crate::model::Model;
use crate::views::TagRecordView;

#[derive(Clap)]
struct Opts {
    working_dir: Option<PathBuf>,
    config_file: Option<PathBuf>,
}

fn main() {
    use globset::Glob;

    let opts = Opts::parse();

    let working_dir =
        match opts.working_dir {
            None => std::env::current_dir().unwrap(),
            Some(working_dir) => working_dir,
        }
    ;

    let config =
        match opts.config_file {
            None => Config::default(),
            Some(config_file_path) => {
                let config_file = File::open(config_file_path).unwrap();
                let reader = BufReader::new(config_file);
                serde_json::from_reader(reader).unwrap()
            },
        }
    ;

    let glob = Glob::new("*.flac").unwrap().compile_matcher();

    let records =
        std::fs::read_dir(&working_dir).unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| glob.is_match(&p))
        .map(|path| {
            let mut record = Record::new();
            let tag = Tag::read_from_path(&path).unwrap();

            for block in tag.blocks() {
                if let Block::VorbisComment(vc_map) = block {
                    for (key, values) in vc_map.comments.iter() {
                        let combined_value = values.join("|");
                        record.insert(key.to_string(), combined_value);
                    }
                }
            }

            let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
            record.insert(str!("FILENAME"), file_name);

            record
        })
        .collect()
    ;

    // let columns = indexmap! {
    //     str!("ARTIST") => ColumnDef {
    //         title: str!("Artist"),
    //         sizing: Sizing::Auto,
    //     },
    //     str!("TITLE") => ColumnDef {
    //         title: str!("Title"),
    //         sizing: Sizing::Auto,
    //     },
    //     str!("FILENAME") => ColumnDef {
    //         title: str!("File Name"),
    //         sizing: Sizing::Auto,
    //     },
    // };
    let columns = config.columns;

    let data = Data::with_data(columns, records);

    let model = Model::with_data(data);

    let main_view = TagRecordView::new(model);

    let mut siv = Cursive::default();

    siv.add_fullscreen_layer(
        Dialog::around(
            main_view
            // .fixed_size((60, 80))
        )
    );

    siv.run();
}
