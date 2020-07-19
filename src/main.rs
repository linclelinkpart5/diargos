
mod consts;
mod cursor;
mod data;
mod model;
mod util;
mod views;

use std::path::PathBuf;

use indexmap::indexmap;
use maplit::hashmap;
use str_macro::str;

use clap::Clap;
use cursive::Cursive;
use cursive::CursiveExt;
use cursive::views::Dialog;
use filetime::FileTime;

use crate::data::ColumnDef;
use crate::data::Data;
use crate::data::Sizing;
use crate::model::Model;
use crate::views::TagRecordView;

#[derive(Clap)]
struct Opts {
    working_dir: Option<PathBuf>,
}

fn main() {
    use rand::seq::SliceRandom;
    use rand::seq::IteratorRandom;

    let opts = Opts::parse();

    let working_dir =
        match opts.working_dir {
            None => std::env::current_dir().unwrap(),
            Some(working_dir) => working_dir,
        }
    ;

    let records =
        glob::glob("/media/poundcake/old_music/Druma Kina - Walking Away EP/*.flac").unwrap()
        .filter_map(Result::ok)
        .filter_map(|path| {
            let metadata = path.metadata().ok()?;
            let mtime = FileTime::from_last_modification_time(&metadata);
            Some(
                hashmap! {
                    str!("path") => path.display().to_string(),
                    str!("mtime") => str!(mtime),
                }
            )
        })
        .collect()
    ;

    let columns = indexmap! {
        str!("path") => ColumnDef {
            title: str!("File Path"),
            sizing: Sizing::Auto,
        },
        str!("mtime") => ColumnDef {
            title: str!("Last Modified"),
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
