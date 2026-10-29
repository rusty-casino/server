use easy_api;
use std::sync::{Arc, Mutex};

mod database;
use database::Database;

mod crypto;

mod games;

fn pong() -> easy_api::EasyResult {
    println!("Pong");
    Ok(())
}

fn bet_wrapper(db: &mut Database, recived: String, func: impl Fn() -> String + Send + Sync + 'static) -> Result<bool, Box<dyn std::error::Error>> {
    let srecv: Vec<&str> = recived.split(' ').collect();

    let (player_tok, market_id, wager): (String, i64, i64) = (
        srecv[0].to_owned(),
        srecv[1].parse().unwrap(),
        srecv[2].parse().unwrap(),
    );

    let player_id = db.get_player_id_token(player_tok)?;

    let bet_id = db.place_bet(player_id, market_id, wager)?;
    let round_id = db.get_round_id(bet_id)?;
    db.play_game(func, round_id)?;
    let win = db.payout_bet(bet_id)?;
    Ok(win)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Arc::new(Mutex::new(Database::new("casino.db")?));

    easy_api::add_tap_api("/ping", pong);

    let cdb = Arc::clone(&mut db);
    easy_api::add_grab_give_api("/bet/coinflip", move |recived| {
        let out = bet_wrapper(&mut cdb.lock().unwrap(), recived, games::coin_flip::play_coin_flip);
        if out.is_err() {
            eprintln!("Bet Failed! {}", out.unwrap_err());
            return Err(500); 
        }
        if out.unwrap() {return Ok("win".to_string());} else {return Ok("loss".to_string());}
    });

    let c2db = Arc::clone(&mut db);
    easy_api::add_grab_give_api("/login", move |recived| {
        let srecv: Vec<&str> = recived.split(' ').collect();
        let (username, password): (String, String) = (srecv[0].to_owned(), srecv[1].to_owned());

        let out = crypto::login(&mut c2db.lock().unwrap(), username, password);
        if out.is_err() {
            eprintln!("Login Failed! {}", out.unwrap_err());
            return Err(500); 
        }
        Ok(out.unwrap())
    });

    let c3db = Arc::clone(&mut db);
    easy_api::add_grab_give_api("/signup", move |recived| {
        let srecv: Vec<&str> = recived.split(' ').collect();
        let (username, password): (String, String) = (srecv[0].to_owned(), srecv[1].to_owned());

        let out = crypto::signup(&mut c3db.lock().unwrap(), username, password);
        if out.is_err() {
            eprintln!("Signup Failed! {}", out.unwrap_err());
            return Err(500); 
        }
        Ok(out.unwrap())
    });

    easy_api::start();
    loop {
        easy_api::park();
    }
}
