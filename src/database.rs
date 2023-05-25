use std::{collections::HashSet, path, sync::Arc};

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serenity::model::prelude::{GuildId, RoleId};
use tokio::fs;

#[derive(Debug)]
pub struct Databases {
    /// A connection to the `guilds` table.
    ///
    /// ```sql
    /// GuildID           BIGINT PRIMARY KEY,
    /// max_content_chars INT,
    /// assignable_roles  BLOB
    /// ```
    pub guilds: r2d2::Pool<SqliteConnectionManager>,
}

impl Databases {
    /// Create connections for databases
    ///
    /// # Errors
    ///
    /// Fails if call to `Self::data_open` fails.
    pub async fn open() -> crate::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            guilds: Self::data_open().await?,
        }))
    }

    /// Return wether a `GuildID` is associated with any rows in `table`.
    ///
    /// # Errors
    ///
    /// Returns errors when
    ///
    /// - preparing or running SQL fails
    ///
    /// # Panics
    ///
    /// This function panics if the database is broken.
    pub fn contains(&self, table: &str, id: GuildId) -> crate::Result<bool> {
        let connection = self.guilds.get().map_err(crate::Error::DatabaseAccessTimeout)?;

        // This SQL returns `1` if it exists, and `0` if it doesn't
        let mut statement = connection
            .prepare(&format!(
                "SELECT EXISTS (SELECT 1 FROM {table} WHERE GuildID={})",
                u64::from(*id.as_inner())
            ))
            .map_err(crate::Error::from)?;
        let exists = statement.query_row([], |row| {
            let value: u32 = row.get(0).unwrap();
            Ok(value)
        });

        Ok(match exists {
            Ok(1) => true,
            Err(e) => return Err(crate::Error::from(e)),
            _ => false,
        })
    }

    /// Creates a row using the provided `GuildID` using default values.
    pub fn guilds_insert_default(&self, guild_id: GuildId) -> crate::Result<()> {
        self.guilds
            .get()
            .map_err(crate::Error::DatabaseAccessTimeout)?
            .execute(
                "INSERT INTO guilds (GuildID, max_content_chars, assignable_roles)
                    VALUES (?1, ?2, ?3)",
                params![
                    u64::from(*guild_id.as_inner()),
                    crate::commands::wiki::Wiki::DEFAULT_MAX_WIKI_LEN,
                    {
                        let assignable_roles: HashSet<RoleId> = HashSet::new();
                        bincode::serialize(&assignable_roles).unwrap()
                    }
                ],
            )
            .map_err(crate::Error::DatabaseAccess)?;

        Ok(())
    }

    /// Create/Open database tables.
    /// Created is the `guilds` table, it's layout is seen below.
    ///
    /// # Tables
    ///
    /// ## guilds
    ///
    /// ```sql
    /// GuildID           BIGINT PRIMARY KEY,
    /// max_content_chars INT,
    /// assignable_roles  BLOB
    /// ```
    /// ## `restricted_words`
    ///
    /// ```sql
    /// word    TEXT PRIMARY KEY,
    /// GuildID BIGINT NOT NULL,
    /// FOREIGN KEY (GuildID) REFERENCES guilds(GuildID)
    /// ```
    ///
    /// # Usage
    ///
    /// ```rust
    /// let guild_settings: Arc<Connection> = guild_data_open(config).await?; 
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Error` when
    ///
    /// - An IO error occurs when trying to create the database.
    /// - A database error occurs when creating the database.
    async fn data_open() -> crate::Result<r2d2::Pool<SqliteConnectionManager>> {
        let database_path = guild_database_path()?;
        fs::create_dir_all(database_path.parent().unwrap())
            .await
            .map_err(crate::Error::Io)?;

        let manager = SqliteConnectionManager::file(database_path);
        let connection = r2d2::Pool::new(manager).map_err(crate::Error::DatabaseOpen)?;

        // Create the `guild_data` and `users` and have each user reference its
        let c = connection.get().map_err(crate::Error::DatabaseAccessTimeout)?;

        c.execute(
            "CREATE TABLE IF NOT EXISTS guilds (
                GuildID           BIGINT PRIMARY KEY,
                max_content_chars INT,
                assignable_roles  BLOB
            )",
            [],
        )
        .map_err(crate::Error::DatabaseAccess)?;

        c.execute(
            "CREATE TABLE IF NOT EXISTS restricted_words (
                word    TEXT PRIMARY KEY,
                GuildID BIGINT NOT NULL,
                FOREIGN KEY (GuildID) REFERENCES guilds(GuildID)

            )",
            [],
        )
        .map_err(crate::Error::DatabaseAccess)?;

        Ok(connection)
    }
}

/// Get the guild database path
#[inline]
fn guild_database_path() -> crate::Result<path::PathBuf> {
    Ok(crate::paths::database_directory()?.join("guildSettings.sqlite"))
}
