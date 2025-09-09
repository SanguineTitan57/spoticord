use diesel::pg::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(connection: &mut PgConnection) -> Result<(), diesel::result::Error> {
    connection
        .run_pending_migrations(MIGRATIONS)
        .map(|_| ())
        .map_err(|_| diesel::result::Error::RollbackTransaction)?; // coarse mapping
    Ok(())
}
