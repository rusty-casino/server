use easy_api;

mod database;
use database::Database;

fn works() {
    println!("It works")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db: Database = Database::new("casino.db")?;

    easy_api::add_tap_api("/ping", works);

    easy_api::start();

    loop {
        easy_api::park();
    }
}
