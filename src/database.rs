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

            CREATE TABLE games (
                id INTEGER PRIMARY KEY,
                market_id 
                name TEXT NOT NULL UNIQUE,
                game_type TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'disabled'))
            );

            CREATE TABLE rounds (
                id INTEGER PRIMARY KEY,
                game_id INTEGER NOT NULL REFERENCES games(id),
                status TEXT NOT NULL DEFAULT 'in_progress'
                    CHECK (status IN (
                        'in_progress', 'finished', 'cancelled'
                    )),
                result TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (game_id) REFERENCES games(id)
                    ON DELETE RESTRICT
            );
            
            CREATE TABLE markets (
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
                created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            );
            ",
        )?;

        Ok(Self { connection })
    }

    pub fn place_bet(&mut self) {
        
    } 

    pub fn transact(tx: &Transaction<'_>, player_id: i64, bet_id: i64, amount: i64, t_type: &str) -> Result<(), Box<dyn std::error::Error>> {
        tx.execute("INSERT INTO transactions (player_id, bet_id, amount, type) VALUES (?1, ?2, ?3, ?4)", params![player_id, bet_id, amount, t_type])?;
        let updated = tx.execute("UPDATE players SET balance = balance + ?1 WHERE id = ?2 AND balance + ?1 >= 0", params![amount, player_id])?;
        if updated != 1 {
            return Err("Player not found or insufficient balance".into());
        }
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
            _round_status,
            result,
            game_name,
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
        
        if result.is_none() {
            return Err("No result availible right now.".into());
        }

        if bet_status != "pending" {
            return Err("Bet has already been settled".into());
        }

        let win: bool = result.unwrap() == selection;

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

    pub fn create_player(&self, name: &str) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO players (username) VALUES (?1)",
            [name],
        )?;

        Ok(self.connection.last_insert_rowid())
    }

    pub fn get_name(&self, id: i64) -> Result<String> {
        self.connection.query_row(
            "SELECT username FROM players WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
    }

    pub fn get_money(&self, id: i64) -> Result<i64> {
        self.connection.query_row(
            "SELECT balance FROM players WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
    }

    pub fn set_money(&self, id: i64, money: i64) -> Result<()> {
        if money < 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Balance cannot be negative".into(),
            ));
        }

        let changed = self.connection.execute(
            "UPDATE players SET balance = ?1 WHERE id = ?2",
            params![money, id],
        )?;

        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        Ok(())
    }

    pub fn update_money(&self, id: i64, dif: i64) -> Result<()> {
        self.set_money(id, self.get_money(id)? + dif)
    }
}
