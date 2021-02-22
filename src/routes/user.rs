use log::error;
use rocket::{
    futures::{future::ready, StreamExt},
    get,
    http::Status,
    routes, Route,
};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use crate::{db::Postgres, guards::UserNameGuard};

pub fn routes() -> Vec<Route> {
    routes![user]
}

#[get("/<username>")]
async fn user<'r>(pg: Postgres<'r>, username: UserNameGuard<'r>) -> Result<Template, Status> {
    if let Some(userid) = userid_from_username(pg, username.as_ref()).await? {
        Ok(Template::render(
            "user",
            UserPage {
                username: username.to_string(),
                repositories: repositories_for_userid(pg, userid).await,
            },
        ))
    } else {
        Err(Status::NotFound)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct UserPage {
    username: String,
    repositories: Vec<Repository>,
}

pub async fn userid_from_username<'a>(
    pg: Postgres<'a>,
    username: &str,
) -> Result<Option<Uuid>, Status> {
    sqlx::query!(
        r#"
        SELECT
            userid
        FROM (
            SELECT
                userid, username
            FROM
                public.users
            WHERE
                username = $1
        ) AS subquery
    "#,
        username,
    )
    .fetch_optional(pg)
    .await
    .map_err(|err| {
        error!("Could not query for user: {}", err);
        Status::InternalServerError
    })
    .map(|result| result.map(|result| result.userid))
}

async fn repositories_for_userid<'a>(pg: Postgres<'a>, userid: Uuid) -> Vec<Repository> {
    sqlx::query!(
        r#"
        SELECT
            repo_name, vcs
        FROM (
            SELECT
                repo_name, vcs, owner_id
            FROM
                public.repositories
            WHERE
                owner_id = $1
        ) AS subquery
        "#,
        userid,
    )
    .fetch(pg)
    .filter_map(|result| {
        ready(match result {
            Err(err) => {
                error!("{}", err);
                None
            }
            Ok(result) => Some(Repository {
                name: result.repo_name,
                vcs: result.vcs,
            }),
        })
    })
    .collect()
    .await
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Repository {
    name: String,
    vcs: String,
}
