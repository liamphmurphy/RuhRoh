extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate curl;
extern crate inputbot;
extern crate rusqlite;

#[macro_use]
extern crate prettytable;
use prettytable::Table;
use prettytable::{color, Attr};

use curl::easy::Easy;
use inputbot::*;
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{stdin, stdout, Write};
use std::path::Path;
use std::process::Command;


#[derive(Deserialize)]
struct Split {
    games: BTreeMap<String, BTreeMap<i32, (String, i32)>>,
}

struct DBSelect {
    boss: String,
}

#[derive(Debug)]
struct Hits {
    boss: String,
    hits: u8,
}

// Display GameSplit object in a nice manner
fn display_highlighted_split(
    game_object: &BTreeMap<i32, (String, i32)>,
    highlight: &i32,
    name: &str,
    hits_vec: &Vec<u8>,
    run_created: &bool,
) {
    let mut table = Table::new();
    let mut total_points = 0;
    let mut vec_index_total = 0;
    // Display game name as a header of sorts
    println!("{}", name);
    // Column names
    table.add_row(row![bFg -> "BOSS", bFg -> "HITS", bFg -> "PB"]);
    let mut vec_index_counter = 0;
    for (index, (boss, hits)) in game_object {
        // Display boss name, current hits, and pb
        if index == highlight {
            table.add_row(row![FB -> boss, rFB -> hits, rFB -> hits_vec[vec_index_counter]]);
        } else {
            table.add_row(row![boss, r -> hits, r -> hits_vec[vec_index_counter]]);
        }

        vec_index_total = vec_index_total + hits_vec[vec_index_counter];
        total_points = total_points + hits;
        vec_index_counter = vec_index_counter + 1;
    }
    // type cast total_points to u8 for comparison to work
    if total_points as u8 > vec_index_total && run_created == &false {
        table.add_row(row![bFr -> "Total:", rbFr -> total_points, rbFr -> vec_index_total]);
    } else {
        table.add_row(row![bFg -> "Total:", rbFg -> total_points, rbFg -> vec_index_total]);
    }
    table.printstd();
    print!("Type 'r' to exit the run and enter a new one.");
}

// This fn returns how many elements are in a games' boss_splits BTreeMap
fn game_map_length(game_object: &BTreeMap<i32, (String, i32)>) -> i32 {
    // Simple increment counter to get length of the run
    let mut map_length = 0;
    for (_, (_, _)) in game_object {
        map_length = map_length + 1;
    }

    return map_length - 1;
}

fn load_json() -> Split {
    let path = Path::new("src/games.json");
    if Path::new(path).exists() == false {
        Command::new("wget")
            .args(&[
                "https://raw.githubusercontent.com/murnux/RuhRoh/master/src/games.json",
                "-P",
                "src",
            ]).output()
            .expect("Error running 'wget'.");
    }
    let file = File::open(path).expect("Error opening games.json, have to close program.");
    // Deserialize into Game struct
    let deserialize_game: Split = serde_json::from_reader(file).unwrap();
    return deserialize_game;
}

// To help reduce lines of code / clutter, this fn takes in a statement string with {} and replaces it with new value.
fn replace_stmt(default: &str, new: &str, characters: &str) -> String {
    let new_stmt = default.replace(characters, new);
    return new_stmt;
}

fn update_pb(game_object: &BTreeMap<i32, (String, i32)>, game_name: &str) {
    if Path::new(DB_PATH).exists() == false {
        println!("Can't find DB, creating new one...");
    }
    // Set up how the SQL statement looks, use {} where value needs to be replaced
    let sql_update = replace_stmt(
        "UPDATE {} Set PBHits = ?1 where Boss = ?2",
        &game_name.trim(),
        "{}",
    );

    let conn = Connection::open(DB_PATH).unwrap();
    for (_, (boss, hits)) in game_object {
        conn.execute(&sql_update, &[hits, boss]).unwrap();
    }
    conn.close().unwrap();
}

fn save_db(game_object: &BTreeMap<i32, (String, i32)>, game_name: &str, hits_vec: &Vec<u8>) {
    let mut index_counter = 0;
    for (_, (_, hits)) in game_object {
        let mut hits_u8 = *hits as u8;
        if &hits_vec[index_counter] > &hits_u8 {
            update_pb(&game_object, &game_name);
        }
        index_counter = index_counter + 1;
    }
}

fn insert_run_into_db(game_object: &BTreeMap<i32, (String, i32)>, game_name: &str) -> bool {
    let mut changes_made = false;
    let sql_insert_change = replace_stmt(
        "INSERT OR IGNORE INTO {} (Boss, PBHits) VALUES (?1, ?2)",
        &game_name.trim(),
        "{}",
    );
    let sql_create_change = replace_stmt(
        "CREATE TABLE IF NOT EXISTS {} (Boss TEXT UNIQUE, PBHits NUMERIC);",
        &game_name.trim(),
        "{}",
    );

    // Replace {} in default string with game name, which is the name of the table.
    let conn = Connection::open(DB_PATH).unwrap();

    conn.execute(&sql_create_change, &[]).unwrap();
    let mut insert_stmt: i32;
    for (_, (boss, hits)) in game_object {
        insert_stmt = conn.execute(&sql_insert_change, &[boss, hits]).unwrap();
        if insert_stmt > 0 {
            changes_made = true;
        }
    }
    conn.close().unwrap();
    return changes_made;
}

fn select_pbs_from_run(game_name: &str) -> Vec<u8> {
    let mut hits_vec = Vec::new();
    let sql_select = replace_stmt("SELECT Boss, PBHits FROM {}", game_name, "{}");

    let conn = Connection::open(DB_PATH).unwrap();

    let mut stmt = conn.prepare(&sql_select).unwrap();
    let hits_iter = stmt
        .query_map(&[], |row| Hits {
            boss: row.get(0),
            hits: row.get(1),
        }).unwrap();
    for result in hits_iter {
        for bosshits in result.into_iter() {
            println!("boss: {}, hits: {}", bosshits.boss, bosshits.hits);
            hits_vec.push(bosshits.hits)
        }
    }

    return hits_vec;
}

fn delete_run_from_db(game_name: &str) {
    let conn = Connection::open(DB_PATH).unwrap();
    let sql_delete_default = "DROP TABLE {}";
    let sql_replace = sql_delete_default.replace("{}", game_name);

    conn.execute(&sql_replace, &[]).unwrap();
}

fn create_run() -> String {
    let game_name: String;
    let mut input = String::new();
    println!("Name of the game.");
    stdin().read_line(&mut input).ok().expect("Couldn't read.");
    game_name = String::from(input.trim());
    let sql_insert = replace_stmt(
        "INSERT OR IGNORE INTO {} (Boss, PBHits) VALUES (?1, ?2)",
        &game_name.trim(),
        "{}",
    );
    let sql_create = replace_stmt(
        "CREATE TABLE IF NOT EXISTS {} (Boss TEXT UNIQUE, PBHits NUMERIC);",
        &game_name.trim(),
        "{}",
    );

    let conn = Connection::open(DB_PATH).unwrap();
    conn.execute(&sql_create, &[]).unwrap();

    let mut counter = 0;
    loop {
        counter = counter + 1;
        println!("Type name of split #{}, or type 'done' to exit.", counter);
        let mut split_input = String::new();
        stdin()
            .read_line(&mut split_input)
            .ok()
            .expect("Couldn't read.");
        if split_input.trim() == "done" {
            break;
        } else {
            conn.execute(&sql_insert, &[&split_input, &"0"]).unwrap();
        }
    }
    return game_name;
}

const DB_PATH: &str = "db/hits.db";

fn main() {
    if Path::new(DB_PATH).exists() == false {
        println!("Can't find DB, creating new one...");
        let dir = fs::create_dir("db");
        let dir = match dir {
            Ok(dir) => dir,
            Err(error) => {
                panic!{"Cannot create missing hits.db in db directory. Exiting."}
            }
        };

        let touch = File::create(DB_PATH);
        let touch = match touch {
            // Handle potential file make errors
            Ok(file) => file,
            Err(error) => {
                panic!{"Cannot create missing hits.db in db directory. Exiting."}
            }
        };
    }
    'change_object: loop {
        print!("Enter next command: ");
        io::Write::flush(&mut io::stdout()).expect("flush failed!");
        // Initialize several variables now for scope reasons
        let object_length: i32;
        let mut input = String::new();
        let mut game_object = BTreeMap::new();
        let list = load_json();
        // Get user input on what they want to do
        stdin().read_line(&mut input).ok().expect("Couldn't read.");

        if input.trim() == "create" {
            create_run();
        }
        // let game = &input.split(" ");
        // Splits up input so the name of the run can be grabbed
        let game_vec: Vec<&str> = (&mut input).split(" ").collect();
        if game_vec.len() == 0 {
        } else {
            if game_vec[0] == "delete" {
                if game_vec.len() > 0 {
                    delete_run_from_db(game_vec[1]);
                }
            }
        }
        let game_target = game_vec[1].trim();

        // Iterates through each run in games.json, and tries to match the run desired from user to one
        let test_key = list.games.contains_key(game_target);

        // If the run name is in games.json, make game_object from that JSON data...
        if test_key == true {
            for (key, value) in list.games {
                // If run selected from run is matched to a run from games.json...
                if game_target == key {
                    for (index, (boss, hit)) in value {
                        // ... create a object for that run that includes the index of each boss (order as they appear in game), boss name, and set hits to 0
                        game_object.insert(index, (boss, hit));
                    }
                }
            }

        // ... but if the run is not in games.json, it is probably a custom run in the DB, so use that instead.
        } else {
            let stmt_change = replace_stmt("SELECT * FROM {}", &game_vec[1], "{}");
            let conn = Connection::open(DB_PATH).unwrap();

            let mut query = conn.prepare(&stmt_change).unwrap();
            let splits_iter = query
                .query_map(&[], |row| DBSelect { boss: row.get(0) })
                .unwrap();
            // To make sure the order is correct in game_object, make an index counter.
            let mut index = 0;
            for result in splits_iter {
                for bosshits in result.into_iter() {
                    game_object.insert(index, (bosshits.boss, 0));
                    index = index + 1; // increment index
                }
            }
        }

        // Get how many splits are in an object for indexing // length purposes
        object_length = game_map_length(&game_object);
        let run_created = insert_run_into_db(&game_object, &game_target);
        // Set up new input for while loop, ownership issues with previous input
        let mut loop_input = String::new();
        let mut hits_vec = Vec::new();
        // Gather any pb's from previous runs
        hits_vec = select_pbs_from_run(&game_target);

        // Stay in while loop until counter is updated from within the loop
        let mut counter = 0;
        'main_counter: while counter == counter {
            print!("{}[2J", 27 as char); // Clears console window
            loop_input = String::from("");
            // Displays the entire run in console, including which run you are on by using the counter variable
            display_highlighted_split(
                &game_object,
                &counter,
                &game_target,
                &hits_vec,
                &run_created,
            );
            stdin()
                .read_line(&mut loop_input)
                .ok()
                .expect("Couldn't read.");

            if loop_input.trim() == "a"
                || loop_input.trim() == "add"
                || loop_input.trim() == "rm"
                || loop_input.trim() == "save"
                || loop_input.trim() == "print"
                || loop_input.trim() == "b"
                || loop_input.trim() == "r"
            {
                // cloning the BTreeMap from GameSplit is a temporary fix to ownership compile errors
                // Note to self for future: apparently this is how higher-language handles this issue!
                for (k, (boss, hit)) in game_object.clone() {
                    if counter == k {
                        if loop_input.trim() == "a" || loop_input.trim() == "add" {
                            // Increment the total hits of a split by 1
                            game_object.insert(counter, (boss.to_string(), hit + 1));
                        } else if loop_input.trim() == "rm" {
                            // If user types 'rm' when current hits is 0, stop it from happening
                            if hit - 1 < 0 {
                                println!("Can't make a hit a negative number.");
                                stdin()
                                    .read_line(&mut loop_input)
                                    .ok()
                                    .expect("Couldn't read.");
                            } else {
                                // Remove one hit from split
                                game_object.insert(counter, (boss.to_string(), hit - 1));
                            }
                        // "Save" is used when a user wants to update their PB
                        } else if loop_input.trim() == "save" {
                            save_db(&game_object, &game_target, &hits_vec)
                        } else if loop_input.trim() == "print" {
                            select_pbs_from_run(&game_target);
                        // Go back a split
                        } else if loop_input.trim() == "b" {
                            counter = counter - 1;
                        } else if loop_input.trim() == "r" {
                            save_db(&game_object, &game_target, &hits_vec);
                            print!("{}[2J", 27 as char); // Clears console window
                            continue 'change_object;
                        }
                    }
                }
            } else {
                if counter >= object_length {
                    counter = 0;
                } else {
                    counter = counter + 1;
                }
            }
        }
    }
}
