use log::error;
use rocket::{
    fairing::{Fairing, Info, Kind},
    futures::{future::BoxFuture, stream::BoxStream},
    request::{FromRequest, Outcome},
    Request, Rocket,
};
use sqlx::{postgres::PgConnectOptions, PgPool};

#[derive(Debug)]
pub struct Postgres<'r> {
    pool: &'r PgPool,
}

impl<'r> Postgres<'r> {
    pub fn fairing() -> PostgresFairing {
        PostgresFairing {}
    }
}

#[async_trait::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for Postgres<'r> {
    type Error = ();

    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let pool: &'r PgPool = request.managed_state().unwrap();
        Outcome::Success(Self { pool })
    }
}

impl<'r, 'c> sqlx::Executor<'c> for &'_ Postgres<'r> {
    type Database = sqlx::Postgres;

    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxStream<
        'e,
        Result<
            either::Either<
                <Self::Database as sqlx::Database>::QueryResult,
                <Self::Database as sqlx::Database>::Row,
            >,
            sqlx::Error,
        >,
    >
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        self.pool.fetch_many(query)
    }

    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<<Self::Database as sqlx::Database>::Row>, sqlx::Error>>
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        self.pool.fetch_optional(query)
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as sqlx::Database>::TypeInfo],
    ) -> BoxFuture<
        'e,
        Result<<Self::Database as sqlx::database::HasStatement<'q>>::Statement, sqlx::Error>,
    >
    where
        'c: 'e,
    {
        self.pool.prepare_with(sql, parameters)
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
    {
        self.pool.describe(sql)
    }
}

pub struct PostgresFairing {}

#[async_trait::async_trait]
impl Fairing for PostgresFairing {
    fn info(&self) -> Info {
        Info {
            name: "sqlx-postgres",
            kind: Kind::Attach,
        }
    }

    async fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        let opts = PgConnectOptions::new();
        match PgPool::connect_with(opts).await {
            Ok(pool) => Ok(rocket.manage(pool)),
            Err(err) => {
                error!("Could not connect to database: {}", err);
                Err(rocket)
            }
        }
    }
}
