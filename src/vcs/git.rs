use rocket::Route;

use crate::{methods, rocket_cgi::CgiScript};

pub struct GitHttpBackend {
    cgi_script: CgiScript,
}

impl GitHttpBackend {
    pub fn new() -> Self {
        let mut base = std::env::var("SRCO2_DATA_DIR").expect("SRCO2_DATA_DIR must be set");
        if !base.ends_with('/') {
            base.push('/');
        }
        let mut project_root = base;
        project_root.push_str("git_repos");

        Self {
            cgi_script: CgiScript::new(
                "git",
                &["http-backend"],
                &[("GIT_PROJECT_ROOT", &project_root)],
                methods![Get, Post],
            ),
        }
    }
}

impl Into<Vec<Route>> for GitHttpBackend {
    fn into(self) -> Vec<Route> {
        self.cgi_script.into()
    }
}
