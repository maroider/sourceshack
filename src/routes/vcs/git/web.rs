use std::{env, path::PathBuf};

use git2::{BranchType, Repository};
use log::warn;
use rocket::{
    get,
    http::Status,
    response::{Redirect, Responder},
    routes, Route,
};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use crate::{
    guards::{AaudStr, UserNameGuard},
    util::ensure_correct_path_separator,
};

use display_tree::{DisplayTree, FileMode};

pub fn routes() -> Vec<Route> {
    routes![view_repository]
}

#[get("/<owner>/<repo>")]
fn view_repository(
    owner: UserNameGuard<'_>,
    repo: AaudStr<'_>,
) -> Result<Template, RedirectOrStatus> {
    let repo = repo.to_string();
    if repo.ends_with(".git") {
        let mut repo = repo;
        repo.truncate(repo.len() - 4);
        return Err(Redirect::permanent(format!("/{}/{}", owner, repo)).into());
    }

    let base = PathBuf::from(ensure_correct_path_separator(
        env::var("SOURCESHACK_DATA_DIR").expect("SOURCESHACK_DATA_DIR is not set"),
    ))
    .join("git_repos");
    let owner_dir = base.join(owner);
    let mut repo_dir = owner_dir.join(&repo);
    let mut ext = repo_dir.extension().unwrap_or_default().to_os_string();
    ext.push("git");
    repo_dir.set_extension(ext);

    match Repository::open_bare(repo_dir) {
        Ok(repository) => {
            let master_branch = repository
                .find_branch("master", BranchType::Local)
                .expect("Could not find branch 'master'");
            let reference = master_branch.get();
            let branch_tip_commit_id = reference.target().unwrap();
            let tree = DisplayTree::new("", &repository, branch_tip_commit_id)
                .items
                .into_iter()
                .map(|item| {
                    let kind = TreeEntryKind::from(item.filemode);
                    let trimmed_commit_message = item.last_commit_message.trim();
                    DisplayTreeEntry {
                        name: item.name,
                        icon: match kind {
                            TreeEntryKind::File => "default_file".to_string(),
                            TreeEntryKind::Directory => "default_folder".to_string(),
                            TreeEntryKind::Symlink => "default_file".to_string(),
                            TreeEntryKind::Gitlink => "file_type_git2".to_string(),
                        },
                        is_not_dir: kind != TreeEntryKind::Directory,
                        commit_message: trimmed_commit_message
                            .split("\n\n")
                            .nth(0)
                            .unwrap_or(trimmed_commit_message)
                            .to_string(),
                    }
                })
                .collect();

            let context = RepositoryInfo {
                owner: owner.as_ref(),
                name: &repo,
                tree,
            };
            Ok(Template::render("repository", context))
        }
        Err(err) => {
            if err.code() == git2::ErrorCode::NotFound {
                dbg!();
                Err(Status::NotFound.into())
            } else {
                warn!("Error in {}/{}: {}", owner, repo, err);
                Err(Status::InternalServerError.into())
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RepositoryInfo<'a> {
    owner: &'a str,
    name: &'a str,
    tree: Vec<DisplayTreeEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DisplayTreeEntry {
    name: String,
    icon: String,
    is_not_dir: bool,
    commit_message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TreeEntryKind {
    File,
    Directory,
    Symlink,
    Gitlink,
}

impl From<FileMode> for TreeEntryKind {
    fn from(from: FileMode) -> Self {
        match from {
            FileMode::File | FileMode::GroupWriteableFile | FileMode::Executable => Self::File,
            FileMode::Directory => Self::Directory,
            FileMode::Symlink => Self::Symlink,
            FileMode::Gitlink => Self::Gitlink,
        }
    }
}

#[derive(Debug, Responder)]
pub enum RedirectOrStatus {
    Redirect(Redirect),
    Status(Status),
}

impl From<Redirect> for RedirectOrStatus {
    fn from(from: Redirect) -> Self {
        Self::Redirect(from)
    }
}

impl From<Status> for RedirectOrStatus {
    fn from(from: Status) -> Self {
        Self::Status(from)
    }
}

mod display_tree {
    use std::{
        collections::HashMap,
        path::{self, Path},
    };

    use git2::{ObjectType, Oid, Repository, TreeEntry};

    #[derive(Debug)]
    pub struct DisplayTree {
        pub items: Vec<DisplayTreeItem>,
    }

    impl DisplayTree {
        /// # Panics
        ///
        /// This function will panice if `path` points to something other than a directory
        pub fn new<P>(path: P, repository: &Repository, commit_id: Oid) -> Self
        where
            P: AsRef<Path>,
        {
            let path = path.as_ref();
            let invalid_prefix = path.components().take(1).find(|component| match component {
                path::Component::Prefix(_)
                | path::Component::RootDir
                | path::Component::CurDir
                | path::Component::ParentDir => true,
                path::Component::Normal(_) => false,
            });
            let normalized_path = invalid_prefix
                .map(|invalid_prefix| path.strip_prefix(invalid_prefix).unwrap())
                .unwrap_or(path);

            let mut revwalk = repository.revwalk().unwrap();
            revwalk.push(commit_id).unwrap();

            let mut oldest_commit_for_object = HashMap::new();

            for older_commit_id in revwalk {
                let older_commit_id = older_commit_id.unwrap();
                let older_commit = repository.find_commit(older_commit_id).unwrap();
                let older_tree = older_commit.tree().unwrap();
                let older_target_tree = {
                    if normalized_path == Path::new("") {
                        older_tree
                    } else {
                        older_tree
                            .get_path(normalized_path)
                            .unwrap()
                            .to_object(repository)
                            .unwrap()
                            .peel_to_tree()
                            .unwrap()
                    }
                };
                for entry in older_target_tree.iter() {
                    if entry.kind() == Some(ObjectType::Blob)
                        || entry.kind() == Some(ObjectType::Tree)
                    {
                        oldest_commit_for_object.insert(entry.id(), older_commit_id);
                    }
                }
            }

            let commit = repository.find_commit(commit_id).unwrap();
            let tree = commit.tree().unwrap();

            let target_tree = {
                if normalized_path == Path::new("") {
                    tree
                } else {
                    tree.get_path(normalized_path)
                        .unwrap()
                        .to_object(repository)
                        .unwrap()
                        .peel_to_tree()
                        .unwrap()
                }
            };

            let items = target_tree
                .iter()
                .filter_map(|entry| {
                    if entry.kind() == Some(ObjectType::Blob)
                        || entry.kind() == Some(ObjectType::Tree)
                    {
                        Some(DisplayTreeItem::new(
                            repository,
                            *oldest_commit_for_object.get(&entry.id()).unwrap(),
                            entry,
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            Self { items }
        }
    }

    // TODO: !
    #[derive(Debug)]
    pub struct DisplayTreeItem {
        pub name: String,
        pub last_commit_id: Oid,
        pub last_commit_message: String,
        pub filemode: FileMode,
    }

    impl DisplayTreeItem {
        fn new(repository: &Repository, last_commit_id: Oid, entry: TreeEntry<'_>) -> Self {
            let last_commit = repository.find_commit(last_commit_id).unwrap();
            Self {
                name: entry.name().unwrap().to_string(),
                last_commit_id,
                last_commit_message: last_commit.message().unwrap().to_string(),
                filemode: FileMode::from_i32(entry.filemode()).unwrap(),
            }
        }
    }

    /// https://stackoverflow.com/a/8347325
    #[allow(clippy::unreadable_literal)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FileMode {
        Directory = 0o040000,
        File = 0o100644,
        GroupWriteableFile = 0o100664,
        Executable = 0o100755,
        Symlink = 0o120000,
        Gitlink = 0o160000,
    }

    impl FileMode {
        pub fn is(self, other: i32) -> bool {
            self as i32 == other
        }

        pub fn from_i32(from: i32) -> Option<Self> {
            if Self::Directory.is(from) {
                Some(Self::Directory)
            } else if Self::File.is(from) {
                Some(Self::File)
            } else if Self::GroupWriteableFile.is(from) {
                Some(Self::GroupWriteableFile)
            } else if Self::Executable.is(from) {
                Some(Self::Executable)
            } else if Self::Symlink.is(from) {
                Some(Self::Symlink)
            } else if Self::Gitlink.is(from) {
                Some(Self::Gitlink)
            } else {
                None
            }
        }
    }
}
