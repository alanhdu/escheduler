use std::path::Path;

use rusqlite::Connection;

pub(crate) struct Database {
    conn: Connection,
}

#[derive(Debug)]
pub(crate) struct Record<'a> {
    pub(crate) name: &'a str,
    pub(crate) weight: u8,
    pub(crate) reps: u16,
}

fn normalize(name: &str) -> &str {
    // Remove whitespace and `({} lbs)` from the name
    name.rsplit_once('(').map(|x| x.0).unwrap_or(name).trim_end()
}

impl Database {
    pub(crate) fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;

        conn.execute(
            "
CREATE TABLE IF NOT EXISTS records (
    id INTEGER PRIMARY KEY AUTOINCREMENT
    , created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    , name TEXT
    , reps INTEGER NOT NULL CHECK (reps >= 1)
    , weight INTEGER NOT NULL CHECK (weight >= 0)
)
        ",
            (),
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS name_weight_reps ON records (name, weight, reps DESC)",
            (),
        )?;
        Ok(Database { conn })
    }

    pub(crate) fn session(&self) -> rusqlite::Result<u32> {
        let sql = "SELECT COUNT(DISTINCT DATE(created_at)) FROM records";
        self.conn.query_one(sql, (), |row| row.get(0))
    }

    pub(crate) fn write(&self, record: &Record<'_>) -> rusqlite::Result<()> {
        let sql =
            "INSERT INTO records (name, weight, reps) VALUES (?1, ?2, ?3)";
        let params = (normalize(record.name), record.weight, record.reps);
        self.conn.execute(sql, params)?;
        Ok(())
    }

    pub(crate) fn best(&self, name: &str, weight: u8) -> rusqlite::Result<u16> {
        let sql =
            "SELECT MAX(reps) FROM records WHERE name = ?1 AND weight = ?2";
        let output: Option<_> =
            self.conn
                .query_one(sql, (normalize(name), weight), |row| row.get(0))?;
        Ok(output.unwrap_or(0))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::Database;

    #[test]
    fn write_and_retrieve() {
        let db = Database::open(":memory:").unwrap();
        db.write(&Record { name: "testing", weight: 0, reps: 1 }).unwrap();
        db.write(&Record { name: "testing", weight: 0, reps: 3 }).unwrap();
        db.write(&Record { name: "testing", weight: 0, reps: 2 }).unwrap();

        db.write(&Record { name: "testing", weight: 5, reps: 10 }).unwrap();
        db.write(&Record { name: "testing", weight: 5, reps: 3 }).unwrap();

        assert_eq!(db.best("testing", 0).unwrap(), 3);
        assert_eq!(db.best("testing", 5).unwrap(), 10);
    }
}
