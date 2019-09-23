use std::{collections::HashMap, env, path::PathBuf};

use git2::Repository;
use log::warn;
use rocket::{get, http::Status, routes, Route};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use crate::util::ensure_correct_path_separator;

pub fn routes() -> Vec<Route> {
    routes![view_repository]
}

#[get("/<owner>/<repo>")]
pub fn view_repository(owner: String, repo: String) -> Result<Template, Status> {
    let base = PathBuf::from(ensure_correct_path_separator(
        env::var("SRCO2_DATA_DIR").expect("SRCO2_DATA_DIR is not set"),
    ))
    .join("git_repos");
    let owner_dir = base.join(&owner);
    let repo_dir = dbg!(owner_dir.join(&repo));

    match Repository::open_bare(repo_dir) {
        Ok(_repository) => {
            let mut context = HashMap::new();
            context.insert(
                "repo_info",
                RepositoryInfo {
                    owner: &owner,
                    name: &repo,
                },
            );
            Ok(Template::render("repository", context))
        }
        Err(err) => {
            if err.code() == git2::ErrorCode::NotFound {
                dbg!();
                Err(Status::NotFound)
            } else {
                warn!("Error in {}/{}: {}", owner, repo, err);
                Err(Status::InternalServerError)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RepositoryInfo<'a> {
    owner: &'a str,
    name: &'a str,
}
