use std::{env, path::PathBuf};

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
    let repo_dir = owner_dir.join(&repo);

    match Repository::open_bare(repo_dir) {
        Ok(repository) => {
            let commit = repository.head().unwrap().peel_to_commit().unwrap();
            // FIXME: Currently panics on empty repos
            let tree: Vec<_> = commit
                .tree()
                .unwrap()
                .iter()
                .map(|entry| {
                    let is_dir = entry.filemode() == FILE_MODE_DIR;
                    let is_file = entry.filemode() == FILE_MODE_FILE;
                    TreeEntry {
                        name: entry.name().unwrap().to_string(),
                        is_dir,
                        is_file,
                    }
                })
                .collect();
            let context = RepositoryInfo {
                owner: &owner,
                name: &repo,
                tree: tree,
            };
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
    tree: Vec<TreeEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TreeEntry {
    name: String,
    is_dir: bool,
    is_file: bool,
}

// FIXME: This is likely incorrect
const FILE_MODE_DIR: i32 = 16384;
const FILE_MODE_FILE: i32 = 33188;
