pub mod error;

mod migrations;
mod models;
mod schema;

use std::sync::Arc;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use error::*;
use models::{Account, LinkRequest, User};
use rand::{distributions::Alphanumeric, Rng};
use rspotify::{clients::BaseClient, Token};
use tokio::task;

/// Helper to retry database operations that fail due to Neon invalidating prepared statements
async fn retry_on_prepared_statement_error<F, R>(operation: F) -> Result<R>
where
    F: Fn() -> Result<R> + Send + 'static + Clone,
    R: Send + 'static,
{
    let op_clone = operation.clone();
    let result = task::spawn_blocking(operation)
        .await
        .map_err(|_| DatabaseError::Diesel(diesel::result::Error::RollbackTransaction))?;

    match result {
        Err(DatabaseError::Diesel(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::Unknown,
            ref info,
        ))) if info
            .message()
            .contains("unnamed prepared statement does not exist") =>
        {
            // Retry once - the prepared statement should be recreated
            task::spawn_blocking(op_clone)
                .await
                .map_err(|_| DatabaseError::Diesel(diesel::result::Error::RollbackTransaction))?
        }
        other => other,
    }
}

#[derive(Clone)]
pub struct Database(Arc<Pool<ConnectionManager<PgConnection>>>);

impl Database {
    pub async fn connect() -> Result<Self> {
        Self::connect_with_url(&spoticord_config::database_url()).await
    }

    pub async fn connect_with_url(database_url: &str) -> Result<Self> {
        // Neon + sync diesel can encounter ephemeral prepared statement invalidation.
        // Disable statement cache so diesel doesn't reuse dropped prepared statements.
        std::env::set_var("DIESEL_STATEMENT_CACHE_SIZE", "0");
        // Use single connection to avoid prepared statement conflicts between connections
        let effective_url = database_url.to_string();
        let manager = ConnectionManager::<PgConnection>::new(effective_url);
        let pool = Pool::builder()
            .max_size(1) // Single connection eliminates prepared statement conflicts
            .connection_timeout(std::time::Duration::from_secs(30))
            .build(manager)
            .map_err(DatabaseError::from)?;

        // Run migrations in blocking thread
        {
            let pool_clone = pool.clone();
            task::spawn_blocking(move || -> Result<()> {
                let mut conn = pool_clone.get().map_err(DatabaseError::from)?;
                migrations::run_migrations(&mut conn).map_err(DatabaseError::from)?;
                Ok(())
            })
            .await
            .map_err(|_| DatabaseError::Diesel(diesel::result::Error::RollbackTransaction))??;
            // map join error
        }

        Ok(Self(Arc::new(pool)))
    }

    // User operations

    pub async fn get_user(&self, user_id: impl AsRef<str>) -> Result<User> {
        use schema::user::dsl::*;

        let pool = self.0.clone();
        let uid = user_id.as_ref().to_string();
        retry_on_prepared_statement_error(move || -> Result<User> {
            let mut connection = pool.get().map_err(DatabaseError::from)?;
            let result = user
                .filter(id.eq(&uid))
                .select(User::as_select())
                .first(&mut connection)?;
            Ok(result)
        })
        .await
    }

    pub async fn create_user(&self, user_id: impl AsRef<str>) -> Result<User> {
        use schema::user::dsl::*;

        let pool = self.0.clone();
        let uid = user_id.as_ref().to_string();
        retry_on_prepared_statement_error(move || -> Result<User> {
            let mut connection = pool.get().map_err(DatabaseError::from)?;
            let result = diesel::insert_into(user)
                .values(id.eq(&uid))
                .returning(User::as_returning())
                .get_result(&mut connection)?;
            Ok(result)
        })
        .await
    }

    pub async fn delete_user(&self, user_id: impl AsRef<str>) -> Result<usize> {
        use schema::user::dsl::*;

        let pool = self.0.clone();
        let uid = user_id.as_ref().to_string();
        retry_on_prepared_statement_error(move || -> Result<usize> {
            let mut connection = pool.get().map_err(DatabaseError::from)?;
            let affected = diesel::delete(user)
                .filter(id.eq(&uid))
                .execute(&mut connection)?;
            Ok(affected)
        })
        .await
    }

    pub async fn get_or_create_user(&self, user_id: impl AsRef<str>) -> Result<User> {
        match self.get_user(&user_id).await {
            Err(DatabaseError::NotFound) => self.create_user(user_id).await,
            result => result,
        }
    }

    pub async fn update_device_name(
        &self,
        user_id: impl AsRef<str>,
        _device_name: impl AsRef<str>,
    ) -> Result<()> {
        use schema::user::dsl::*;

        let pool = self.0.clone();
        let uid = user_id.as_ref().to_string();
        let dname = _device_name.as_ref().to_string();
        retry_on_prepared_statement_error(move || -> Result<()> {
            let mut connection = pool.get().map_err(DatabaseError::from)?;
            diesel::update(user)
                .filter(id.eq(&uid))
                .set(device_name.eq(&dname))
                .execute(&mut connection)?;
            Ok(())
        })
        .await
    }

    // Account operations

    pub async fn get_account(&self, _user_id: impl AsRef<str>) -> Result<Account> {
        use schema::account::dsl::*;

        let pool = self.0.clone();
        let uid = _user_id.as_ref().to_string();
        retry_on_prepared_statement_error(move || -> Result<Account> {
            let mut connection = pool.get().map_err(DatabaseError::from)?;
            let result = account
                .select(Account::as_select())
                .filter(user_id.eq(&uid))
                .first(&mut connection)?;
            Ok(result)
        })
        .await
    }

    pub async fn delete_account(&self, _user_id: impl AsRef<str>) -> Result<usize> {
        use schema::account::dsl::*;

        let pool = self.0.clone();
        let uid = _user_id.as_ref().to_string();
        retry_on_prepared_statement_error(move || -> Result<usize> {
            let mut connection = pool.get().map_err(DatabaseError::from)?;
            let affected = diesel::delete(account)
                .filter(user_id.eq(&uid))
                .execute(&mut connection)?;
            Ok(affected)
        })
        .await
    }

    pub async fn update_session_token(
        &self,
        _user_id: impl AsRef<str>,
        _session_token: Option<String>,
    ) -> Result<()> {
        use schema::account::dsl::*;

        let pool = self.0.clone();
        let uid = _user_id.as_ref().to_string();
        let token_opt = _session_token.clone();
        retry_on_prepared_statement_error(move || -> Result<()> {
            let mut connection = pool.get().map_err(DatabaseError::from)?;
            diesel::update(account)
                .filter(user_id.eq(&uid))
                .set(session_token.eq(token_opt.as_deref()))
                .execute(&mut connection)?;
            Ok(())
        })
        .await
    }

    // Request operations

    pub async fn get_request(&self, _user_id: impl AsRef<str>) -> Result<LinkRequest> {
        use schema::link_request::dsl::*;

        let pool = self.0.clone();
        let uid = _user_id.as_ref().to_string();
        retry_on_prepared_statement_error(move || -> Result<LinkRequest> {
            let mut connection = pool.get()?;
            let result = link_request
                .select(LinkRequest::as_select())
                .filter(user_id.eq(&uid))
                .first(&mut connection)?;
            Ok(result)
        })
        .await
    }

    /// Create a new link request that expires after an hour
    pub async fn create_request(&self, _user_id: impl AsRef<str>) -> Result<LinkRequest> {
        use schema::link_request::dsl::*;

        let pool = self.0.clone();
        let uid = _user_id.as_ref().to_string();
        task::spawn_blocking(move || -> Result<LinkRequest> {
            let mut connection = pool.get()?;
            let _token: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect();
            let _expires = (Utc::now() + Duration::hours(1)).naive_utc();
            let request = diesel::insert_into(link_request)
                .values((user_id.eq(&uid), token.eq(&_token), expires.eq(_expires)))
                .on_conflict(user_id)
                .do_update()
                .set((token.eq(&_token), expires.eq(_expires)))
                .returning(LinkRequest::as_returning())
                .get_result(&mut connection)?;
            Ok(request)
        })
        .await
        .map_err(|_| DatabaseError::Diesel(diesel::result::Error::RollbackTransaction))?
    }

    // Special operations

    /// Retrieve a user's Spotify access token. This token, if expired, will automatically be refreshed
    /// using the refresh token stored in the database. If this succeeds, the access token will be updated.
    pub async fn get_access_token(&self, _user_id: impl AsRef<str>) -> Result<String> {
        use schema::account::dsl::*;

        let uid = _user_id.as_ref().to_string();
        let pool = self.0.clone();
        let mut result: Account = task::spawn_blocking({
            let pool = pool.clone();
            let uid = uid.clone();
            move || -> Result<Account> {
                let mut connection = pool.get().map_err(DatabaseError::from)?;
                let result = account
                    .filter(user_id.eq(&uid))
                    .select(Account::as_select())
                    .first(&mut connection)?;
                Ok(result)
            }
        })
        .await
        .map_err(|_| DatabaseError::Diesel(diesel::result::Error::RollbackTransaction))??;

        if result.expired_offset(Duration::minutes(1)) {
            let refresh_token_value = result.refresh_token.clone();
            let spotify = spoticord_config::get_spotify(Token {
                refresh_token: Some(refresh_token_value),
                ..Default::default()
            });

            let token = match spotify.refetch_token().await {
                Ok(Some(token)) => token,
                _ => {
                    self.delete_account(&uid).await.ok();
                    return Err(DatabaseError::RefreshTokenFailure);
                }
            };

            let pool2 = pool.clone();
            let uid2 = uid.clone();
            let access_token_val = token.access_token.clone();
            let refresh_token_val = token.refresh_token.clone();
            let expires_val = token
                .expires_at
                .expect("token expires_at is none, we broke time")
                .naive_utc();

            result = task::spawn_blocking(move || -> Result<Account> {
                let mut connection = pool2.get().map_err(DatabaseError::from)?;
                let updated = diesel::update(account)
                    .filter(user_id.eq(&uid2))
                    .set((
                        access_token.eq(&access_token_val),
                        refresh_token.eq(refresh_token_val.as_deref().unwrap_or("")),
                        expires.eq(&expires_val),
                    ))
                    .returning(Account::as_returning())
                    .get_result(&mut connection)?;
                Ok(updated)
            })
            .await
            .map_err(|_| DatabaseError::Diesel(diesel::result::Error::RollbackTransaction))??;
        }

        Ok(result.access_token)
    }
}
