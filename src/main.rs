extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate rusqlite;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::{stdin, stdout, Write};
use std::fs::File;
use std::path::Path;
use rusqlite::Connection;
use std::mem;

#[derive(Deserialize)]
struct Game {
    games: HashMap<String, BTreeMap<i32, (String, i32)>>
}

#[derive(Deserialize)]
struct GameSplit {
    boss_splits: BTreeMap<String, i32>,
    //boss_splits: BTreeMap<i32, (String, i32)>, // Using BTreeMap because the order of bosses is important and cannot be unordered.
}

#[derive(Debug)]
struct Hits {
    boss: String,
    hits: u8
}

// Display GameSplit object in a nice manner
fn display_highlighted_split(game_object: &BTreeMap<i32, (String, i32)>, highlight: &i32, name: &String, hits_vec: &Vec<u8>) {
    // Display game name as a header of sorts
    println!("{}\n_________________________________________\n", name);
    // Column names
    println!("BOSS\t   HITS\t   PB");
    let mut vec_index_counter = 0;
    for (index, (boss, hits)) in game_object {
        // Display boss name, current hits, and pb 
        print!("{} | {} | {}", boss, hits, hits_vec[vec_index_counter]);
        // Highlight the split that is selected
        if index == highlight {
            print!(" <-----");
        }
        println!("\n------------------------------");
        vec_index_counter = vec_index_counter + 1;
    }
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

fn load_json() -> Game {
    let path = Path::new("src/games.json");
    let file = File::open(path).expect("Error opening games.json, have to close program.");
    // Deserialize into Game struct
    let deserialize_game: Game = serde_json::from_reader(file).unwrap();
    return deserialize_game;
}

fn update_pb(game_object: &BTreeMap<i32, (String, i32)>, game_name: &String) {
    let db_path = "src/hits.db";
    if Path::new(db_path).exists() == false {
        println!("Can't find DB, creating new one...");
    }
    // Set up how the SQL statement looks, use {} where value needs to be replaced
    let sql_default = "UPDATE {} Set PBHits = ?1 where Boss = ?2";
    
    // Replace {} in default string with game name, which is the name of the table.
    let sql_replace = sql_default.replace("{}", &game_name); 

    let conn = Connection::open(db_path).unwrap();
    for (_, (boss, hits)) in game_object {
        conn.execute(&sql_replace, &[hits, boss]).unwrap();
    }
    conn.close().unwrap();
}

fn insert_run_into_db(game_object: &BTreeMap<i32, (String, i32)>, game_name: &String) {
    let db_path = "src/hits.db";
    if Path::new(db_path).exists() == false {
        println!("Can't find DB, creating new one...");
    }
    // Set up how the SQL statement looks, use {} where value needs to be replaced
    let sql_insert_default = "INSERT OR IGNORE INTO {} (Boss, PBHits) VALUES (?1, ?2)";
    let sql_create_default = "CREATE TABLE IF NOT EXISTS {} (Boss TEXT UNIQUE, PBHits NUMERIC);";

    // Replace {} in default string with game name, which is the name of the table.
    let sql_insert_replace = sql_insert_default.replace("{}", &game_name.trim()); 
    let sql_create_replace = sql_create_default.replace("{}", &game_name.trim());
    let conn = Connection::open(db_path).unwrap();
    println!("SQL CREATE: \n{}", sql_create_replace);


    conn.execute(&sql_create_replace, &[]).unwrap();
    for (_, (boss, hits)) in game_object {
        conn.execute(&sql_insert_replace, &[boss, hits]).unwrap();
    }
    conn.close().unwrap();
}

fn select_pbs_from_run (game_name: &String) -> Vec<u8> {
    let mut hits_vec = Vec::new();
    let db_path = "src/hits.db";
    let sql_select_default = "SELECT Boss, PBHits FROM {}";
    let sql_select_replace = sql_select_default.replace("{}", game_name);

    let conn = Connection::open(db_path).unwrap();

    let mut stmt = conn.prepare(&sql_select_replace).unwrap();
    let hits_iter = stmt.query_map(&[], |row| Hits {boss: row.get(0), hits: row.get(1)}).unwrap();
    for result in hits_iter {
       // println!("Hit: {:?}", hit.unwrap());
        for bosshits in result.into_iter() {
            println!("boss: {}, hits: {}", bosshits.boss, bosshits.hits);
            hits_vec.push(bosshits.hits)
        }
    }
    
    return hits_vec;
}
 

fn main() {
    // Initialize several variables now for scope reasons
    let mut counter = 0;
    let object_length: i32;
    let mut input = String::new();
    let mut game_object = BTreeMap::new();
    let game_name: String;

    // Loads up games.json, puts data into Game struct
    let list = load_json();
    
    print!("Enter next command: ");

    // Get user input on what they want to do
    stdin().read_line(&mut input).ok().expect("Couldn't read.");
   // let game = &input.split(" ");
   // Splits up input so the name of the run can be grabbed
    let game_vec: Vec<&str> = (&mut input).split(" ").collect();
    
    // Iterates through each run in games.json, and tries to match the run desired from user to one
    for (key, value) in list.games {
        println!("key: {}", key);
        // If run selected from run is matched to a run from games.json...
        if game_vec[1].trim() == key {
            for (index, (boss, hit)) in value {
                // ... create a object for that run that includes the index of each boss (order as they appear in game), boss name, and set hits to 0
                game_object.insert(index, (boss, hit));
            }
        }
    }

    // Get how many splits are in an object for indexing // length purposes
    object_length = game_map_length(&game_object);
    game_name = String::from(game_vec[1]);
        loop {
            insert_run_into_db(&game_object, &game_name);
            // Set up new input for while loop, ownership issues with previous input
            let mut loop_input = String::new(); 
            let mut hits_vec = Vec::new();
            // Gather any pb's from previous runs
            hits_vec = select_pbs_from_run(&game_name);
            // Stay in while loop until counter is updated from within the loop
            while counter == counter {
                print!("{}[2J", 27 as char); // Clears console window
                loop_input = String::from("");
                // Displays the entire run in console, including which run you are on by using the counter variable
                display_highlighted_split(&game_object, &counter, &game_name, &hits_vec);
                stdin().read_line(&mut loop_input).ok().expect("Couldn't read.");
                if loop_input.trim() == "add" || loop_input.trim() == "rm" || loop_input.trim() == "save" || loop_input.trim() == "print" || loop_input.trim() == "b" {
                    // cloning the BTreeMap from GameSplit is a temporary fix to ownership compile errors 
                    // Note to self for future: apparently this is how higher-language handles this issue!
                    for (k, (boss, hit)) in game_object.clone() {
                        if counter == k {
                            if loop_input.trim() == "add" {
                                // Increment the total hits of a split by 1
                                game_object.insert(counter, (boss.to_string(), hit + 1));
                            } else if loop_input.trim() == "rm"{
                                // If user types 'rm' when current hits is 0, stop it from happening
                                if hit - 1 < 0 {
                                    println!("Can't make a hit a negative number.");
                                    stdin().read_line(&mut loop_input).ok().expect("Couldn't read.");
                                } else {
                                    // Remove one hit from split
                                    game_object.insert(counter, (boss.to_string(), hit - 1));
                                }
                                // "Save" is used when a user wants to update their PB
                            } else if loop_input.trim() == "save" {
                                let mut index_counter = 0;
                                for (index, (_, hits)) in &game_object {
                                    let mut hits_u8 = *hits as u8;
                                    if &hits_vec[index_counter] > &hits_u8 {
                                        update_pb(&game_object, &game_name);
                                    }
                                    index_counter = index_counter + 1;
                                }
                                update_pb(&game_object, &game_name);
                            } else if loop_input.trim() == "print" {
                                select_pbs_from_run(&game_name);
                            // Go back a split
                            } else if loop_input.trim() == "b" {
                                counter = counter - 1;
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

