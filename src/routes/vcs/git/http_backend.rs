use std::path::{Path, PathBuf};

use rocket::{
    handler::{Handler, Outcome},
    http::Method,
    Config, Data, Request, Response, Route, State,
};

use crate::cgi::CgiScript;

#[derive(Clone, Debug)]
pub struct GitHttpBackend {
    repo_dir: PathBuf,
}

impl GitHttpBackend {
    pub fn new<P: AsRef<Path>>(repo_dir: P) -> Self {
        let repo_dir = repo_dir.as_ref().to_path_buf();
        Self { repo_dir }
    }
}

impl Handler for GitHttpBackend {
    fn handle<'r>(&self, request: &'r Request, data: Data) -> Outcome<'r> {
        // TODO: Handle the error case.
        let config: State<Config> = request.guard().unwrap();

        let path_translated = dbg!(&self.repo_dir).join(request.uri().segments().enumerate().fold(
            String::new(),
            |mut path, (i, segment)| {
                if i > 0 {
                    #[cfg(windows)]
                    path.push('\\');
                    #[cfg(not(windows))]
                    path.push('/');
                }
                path.push_str(segment);
                if i == 1 {
                    if !path.ends_with(".git") {
                        path.push_str(".git");
                    }
                }
                path
            },
        ));

        Outcome::from(
            request,
            dbg!(dbg!(CgiScript::new("git", &["http-backend"], &[])
                .server_software("rocket")
                .server_name(&config.address.to_string())
                .server_port(&config.port.to_string())
                .request_method(request.method().as_str())
                .query_string(request.uri().query().unwrap_or(""))
                .remote_addr(
                    &request
                        .client_ip()
                        .map(|ip| ip.to_string())
                        .unwrap_or_default()
                )
                // TODO: Do .git normalization to PATH_INFO
                .path_info(request.uri().path())
                .path_translated(&path_translated.to_str().unwrap().replace('\\', "/"))
                .content_type(
                    &request
                        .content_type()
                        .map(|ct| ct.to_string())
                        .unwrap_or_default()
                ))
            .run(data.open()))
            .map(|response| {
                let response: Response = response.into();
                response
            }),
        )
    }
}

impl Into<Vec<Route>> for GitHttpBackend {
    fn into(self) -> Vec<Route> {
        vec![
            Route::new(Method::Get, "/<user>/<repo>/info/refs", self.clone()),
            Route::new(Method::Post, "/<user>/<repo>/git-upload-pack", self.clone()),
            Route::new(
                Method::Post,
                "/<user>/<repo>/git-receive-pack",
                self.clone(),
            ),
        ]
    }
}
