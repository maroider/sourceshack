use std::path::{Path, PathBuf};

use rocket::{get, post, routes, Route};

use crate::cgi::CgiScript;

pub struct GitHttpBackend {
    repo_dir: PathBuf,
}

impl GitHttpBackend {
    pub fn new<P: AsRef<Path>>(repo_dir: P) -> Self {
        let repo_dir = repo_dir.as_ref().to_path_buf();
        Self { repo_dir }
    }
}

impl Into<Vec<Route>> for GitHttpBackend {
    fn into(self) -> Vec<Route> {
        routes![]
    }
}
