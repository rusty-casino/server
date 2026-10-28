use easy_api;
use std::sync::{Arc, Mutex};

mod database;
use database::Database;

mod crypto;

mod games;

fn works() {
    println!("It works")
}

fn game_wrapper(db: &Database, market_id: i64, wager: i64, func: impl Fn() -> String + Send + Sync + 'static) -> Result<(), Box<dyn std::error::Error>> {
    let bet_id = db.place_bet(player_id, market_id, wager)?;
    let round_id = db.get_round_id(bet_id)?;
    db.play_game(func, round_id);
    db.payout_bet(bet_id);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Arc::new(Mutex::new(Database::new("casino.db")?));

    easy_api::add_tap_api("/ping", works);

    let cdb = Arc::clone(&db);

    easy_api::add_grab_api("/bet/coinflip", move |recived| {
        let srecv: Vec<&str> = recived.split(' ').collect();

        let (market_id, wager): (i64, i64) = (
            srecv[0].parse().unwrap(),
            srecv[1].parse().unwrap(),
        );

        let cdb = cdb.lock().unwrap();

        game_wrapper(
            &cdb,
            market_id,
            wager,
            games::coin_flip::play_coin_flip,
        );
    });

    let c2db = Arc::clone(&mut db);

    easy_api::add_grab_api("/login", move |recived| {
        let srecv: Vec<&str> = recived.split(' ').collect();

        let (username, password): (String, String) = (srecv[0].to_owned(), srecv[1].to_owned());

        let mut c2db = c2db.lock().unwrap();

        crypto::login(&mut c2db, username, password);
    });

    easy_api::start();

    loop {
        easy_api::park();
    }
}
