use easy_api;

mod database;
use database::Database;

mod games;

fn works() {
    println!("It works")
}

fn game_wrapper() {
    
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db: Database = Database::new("casino.db")?;

    easy_api::add_tap_api("/ping", works);

    easy_api::start();

    loop {
        easy_api::park();
    }
}
