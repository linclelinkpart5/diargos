
mod consts;
mod cursor;
mod data;
mod model;
mod util;
mod views;

use std::path::PathBuf;

use indexmap::indexmap;
use str_macro::str;

use clap::Clap;
use cursive::Cursive;
use cursive::CursiveExt;
use cursive::views::Dialog;
use metaflac::Tag;
use metaflac::Block;

use crate::data::ColumnDef;
use crate::data::Data;
use crate::data::Record;
use crate::data::Sizing;
use crate::model::Model;
use crate::views::TagRecordView;

#[derive(Clap)]
struct Opts {
    working_dir: Option<PathBuf>,
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

    let columns = indexmap! {
        str!("ARTIST") => ColumnDef {
            title: str!("Artist"),
            sizing: Sizing::Auto,
        },
        str!("TITLE") => ColumnDef {
            title: str!("Title"),
            sizing: Sizing::Auto,
        },
        str!("FILENAME") => ColumnDef {
            title: str!("File Name"),
            sizing: Sizing::Auto,
        },
    };

    // let fave_foods = vec![
    //     "pizza",
    //     "steak",
    //     "lasagna",
    //     "tacos",
    //     "burritos",
    //     "chicken",
    //     "burgers",
    //     "waffles",
    //     "sushi",
    //     "curry",
    //     "ice cream",
    //     "brownies",
    //     "popcorn",
    //     "burritos",
    //     "fried rice",
    // ];

    // let names = vec![
    //     "Raina Salas",
    //     "Mariah Hernandez",
    //     "Kadin Rivas",
    //     "Osvaldo Hebert",
    //     "Adrien Lutz",
    //     "Peyton Mckenzie",
    //     "Valentin Nixon",
    //     "Greta Miles",
    //     "Cameron French",
    //     "Jayden Romero",
    //     "Alden Conrad",
    //     "Peter King",
    //     "Jake Duncan",
    //     "Shaun Barr",
    //     "Danna Shannon",
    //     "日本人の氏名",
    // ];

    // let num_records = 100;

    // let mut rng = rand::thread_rng();

    // let records =
    //     (1..=num_records)
    //     .map(|i| {
    //         let mut m = hashmap! {
    //             // str!("index") => str!(i),
    //             str!("name") => names.choose(&mut rng).unwrap().to_string(),
    //             str!("age") => str!((18..=70).choose(&mut rng).unwrap()),
    //             str!("score") => str!((0..=100).choose(&mut rng).unwrap()),
    //             str!("is_outgoing") => str!(rand::random::<bool>()),
    //         };

    //         if i >= num_records / 2 {
    //             m.insert(str!("fave_food"), fave_foods.choose(&mut rng).unwrap().to_string());
    //         }

    //         m
    //     })
    //     .collect::<Vec<_>>()
    // ;

    // let columns = indexmap! {
    //     str!("index") => ColumnDef {
    //         title: str!("Index"),
    //         sizing: Sizing::Auto,
    //     },
    //     str!("name") => ColumnDef {
    //         title: str!("日本人の氏名"),
    //         sizing: Sizing::Fixed(6),
    //     },
    //     str!("age") => ColumnDef {
    //         title: str!("Age"),
    //         sizing: Sizing::Fixed(90),
    //     },
    //     str!("fave_food") => ColumnDef {
    //         title: str!("Favorite Food"),
    //         sizing: Sizing::Fixed(90),
    //     },
    //     str!("score") => ColumnDef {
    //         title: str!("Score"),
    //         sizing: Sizing::Auto,
    //     },
    //     str!("is_outgoing") => ColumnDef {
    //         title: str!("Is Outgoing?"),
    //         sizing: Sizing::Fixed(90),
    //     },
    // };

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
