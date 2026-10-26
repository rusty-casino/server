//use anyhow::Ok;
use rusqlite::{params, Connection, Result};

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
                balance  INTEGER NOT NULL DEFAULT 1000
                         CHECK (balance >= 0)
            );

            CREATE TABLE IF NOT EXISTS bets (
                id         INTEGER PRIMARY KEY,
                player_id  INTEGER NOT NULL REFERENCES players(id),
                game       TEXT NOT NULL,
                amount     INTEGER NOT NULL CHECK (amount > 0),
                payout     INTEGER NOT NULL DEFAULT 0,
                status     TEXT NOT NULL DEFAULT 'pending'
                           CHECK (status IN ('pending', 'won', 'lost', 'cancelled')),
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS transactions (
                id        INTEGER PRIMARY KEY,
                player_id INTEGER NOT NULL REFERENCES players(id),
                bet_id    INTEGER REFERENCES bets(id),
                amount    INTEGER NOT NULL,
                type      TEXT NOT NULL
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;

        Ok(Self { connection })
    }

    pub fn new_transaction(&self, id: i64, ) {
        
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
