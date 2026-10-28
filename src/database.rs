use rusqlite::{Connection, Result, Transaction, params};

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let connection = Connection::open(path)?;

        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS players (
                id       INTEGER PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                balance  INTEGER NOT NULL DEFAULT 10000
                         CHECK (balance >= 0)
            );

            CREATE TABLE IF NOT EXISTS games (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                game_type TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'disabled'))
            );

            CREATE TABLE IF NOT EXISTS rounds (
                id INTEGER PRIMARY KEY,
                game_id INTEGER NOT NULL REFERENCES games(id)
                    ON DELETE RESTRICT,
                status TEXT NOT NULL DEFAULT 'in_progress'
                    CHECK (status IN (
                        'in_progress', 'finished', 'cancelled'
                    )),
                result TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE IF NOT EXISTS markets (
                id INTEGER PRIMARY KEY,
                game_id INTEGER NOT NULL REFERENCES games(id)
                    ON DELETE RESTRICT,
                selection TEXT NOT NULL,
                odds REAL NOT NULL CHECK (odds > 1.0),

                UNIQUE (game_id, selection)
            );

            CREATE TABLE IF NOT EXISTS bets (
                id         INTEGER PRIMARY KEY,
                player_id  INTEGER NOT NULL REFERENCES players(id),
                market_id  INTEGER NOT NULL REFERENCES markets(id),
                round_id   INTEGER NOT NULL REFERENCES rounds(id),
                game       TEXT NOT NULL,
                amount     INTEGER NOT NULL CHECK (amount > 0),
                payout     INTEGER NOT NULL DEFAULT 0,
                status     TEXT NOT NULL DEFAULT 'pending'
                           CHECK (status IN ('pending', 'won', 'lost', 'cancelled')),
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS transactions (
                id           INTEGER PRIMARY KEY,
                player_id    INTEGER NOT NULL REFERENCES players(id),
                bet_id       INTEGER REFERENCES bets(id),
                amount       INTEGER NOT NULL,
                type         TEXT NOT NULL,
                created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;

        Ok(Self { connection })
    }

    fn calculate_payout(wager: i64, odds: f64) -> Result<i64> {
        if wager <= 0 || !odds.is_finite() || odds <= 1.0 {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let payout = (wager as f64) * odds;

        if !payout.is_finite() || payout > i64::MAX as f64 {
            return Err(rusqlite::Error::InvalidQuery);
        }

        Ok(payout.floor() as i64)
    }
    
    fn transact(tx: &Transaction<'_>, player_id: i64, bet_id: i64, amount: i64, t_type: &str) -> Result<(), Box<dyn std::error::Error>> {
        tx.execute("INSERT INTO transactions (player_id, bet_id, amount, type) VALUES (?1, ?2, ?3, ?4)", params![player_id, bet_id, amount, t_type])?;
        let updated = tx.execute("UPDATE players SET balance = balance + ?1 WHERE id = ?2 AND balance + ?1 >= 0", params![amount, player_id])?;
        if updated != 1 {
            return Err("Player not found or insufficient balance".into());
        }
        Ok(())
    }

    pub fn place_bet(&mut self, player_id: i64, market_id: i64, wager: i64) -> Result<(), Box<dyn std::error::Error>> {
        if wager <= 0 {
            return Err("Wager must be positive".into());
        }

        let tx = self.connection.transaction()?;
        let (game_id, bet_on, odds, game_name): (i64, String, f64, String) = tx.query_row(
            "SELECT m.game_id, m.selection, m.odds, g.name FROM markets m JOIN games g ON m.game_id = g.id WHERE m.id = ?1 AND game.status = 'active'",
            params![market_id], |row| { Ok(( row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)? )) }
        )?;
        tx.execute("INSERT INTO rounds (game_id) VALUES (?1)", params![game_id])?;
        let round_id = tx.last_insert_rowid();
        let payout = Self::calculate_payout(wager, odds)?;
        tx.execute("INSERT INTO bets (player_id, market_id, round_id, game, amount, payout) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![player_id, market_id, round_id, game_name, wager, payout]
        )?;
        let bet_id = tx.last_insert_rowid();
        let transact_type = format!("Wager for Game: /{}/ for a bet on /{}/", game_name, bet_on);
        Self::transact(&tx, player_id, bet_id, -wager, &transact_type)?;
        tx.commit()?;
        Ok(())
    } 

    pub fn payout_bet(&mut self, bet_id: i64) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.connection.transaction()?;

        let (
            player_id,
            _amount,
            bet_status,
            payout,
            selection,
            _odds,
            round_status,
            result,
            game_name
        ): (i64, i64, String, i64, String, f64, String, Option<String>, String) =
            tx.query_row(
                "SELECT
                    b.player_id,
                    b.amount,
                    b.status,
                    b.payout,
                    m.selection,
                    m.odds,
                    r.status,
                    r.result,
                    g.name
                FROM bets b
                JOIN markets m ON m.id = b.market_id
                JOIN rounds r ON r.id = b.round_id
                JOIN games g ON g.id = m.game_id
                WHERE b.id = ?1
                AND r.game_id = m.game_id",
                [bet_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )?;

        if round_status != "finished" {
            return Err("Round is not over yet.".into())
        }

        if result.is_none() {
            return Err("No result availible right now.".into());
        }

        if bet_status != "pending" {
            return Err("Bet has already been settled".into());
        }

        let result = result.ok_or("No result available")?;
        let win = result == selection;

        if !win {
            tx.execute("UPDATE bets SET status = 'lost' WHERE id = ?1", [bet_id])?;
        } else {
            tx.execute("UPDATE bets SET status = 'won' WHERE id = ?1", [bet_id])?;
            let bet_type = format!("Won Game: /{}/ by betting on /{}/", game_name, selection);
            Self::transact(&tx, player_id, bet_id, payout, &bet_type)?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn play_game(&mut self, game_func: impl Fn() -> String + Send + Sync + 'static, round_id: i64) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.connection.transaction()?;
        let res = game_func();
        
        let updated = tx.execute(
            "UPDATE rounds
            SET status = 'finished', result = ?1
            WHERE id = ?2 AND status = 'in_progress'",
            params![res, round_id],
        )?;

        if updated != 1 {
            return Err("Round could not be finished".into());
        }

        tx.commit()?;
        Ok(())
    }
}
