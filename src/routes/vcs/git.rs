use std::{convert::TryFrom, env, path::PathBuf};

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
                    let kind = FileType::try_from(entry.filemode())
                        .map(|fm| fm as i8)
                        .unwrap();
                    TreeEntry {
                        name: entry.name().unwrap().to_string(),
                        kind,
                        is_not_dir: kind != FileType::Directory as i8,
                    }
                })
                .collect();
            let context = RepositoryInfo {
                owner: &owner,
                name: &repo,
                tree,
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
    kind: i8,
    is_not_dir: bool,
}

enum FileType {
    File = 1,
    Directory = 2,
    Symlink = 3,
    Gitlink = 4,
}

impl TryFrom<i32> for FileType {
    type Error = FileTypeError;

    fn try_from(fm: i32) -> Result<Self, Self::Error> {
        // https://unix.stackexchange.com/questions/450480/file-permission-with-six-bytes-in-git-what-does-it-mean/450488#450488
        let ft = fm >> 12;
        if ft ^ 0b1000 == 0 {
            Ok(Self::File)
        } else if ft ^ 0b0100 == 0 {
            Ok(Self::Directory)
        } else if ft ^ 0b1010 == 0 {
            Ok(Self::Symlink)
        } else if ft ^ 0b1110 == 0 {
            Ok(Self::Gitlink)
        } else {
            Err(FileTypeError(fm))
        }
    }
}

#[derive(Debug)]
struct FileTypeError(i32);

impl std::fmt::Display for FileTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Invalid file type: {}", self.0)
    }
}

impl std::error::Error for FileTypeError {}
