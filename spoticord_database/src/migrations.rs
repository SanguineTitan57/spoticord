use diesel::pg::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(connection: &mut PgConnection) -> Result<(), diesel::result::Error> {
    match connection.run_pending_migrations(MIGRATIONS) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Migration error: {:?}", e);
            Err(diesel::result::Error::RollbackTransaction)
        }
    }
}
