use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use flate2::read::GzDecoder;
use log::trace;
use rocket::{
    handler::{Handler, Outcome},
    http::{Method, Status},
    Config, Data, Request, Response, Route, State,
};

/// # Issues
///
/// * Currently can't forward username and password portions of the request's URI:
///   [rocket#998](https://github.com/SergioBenitez/Rocket/issues/998)
/// * Doesn't support http and https at the same time or multiple simultaneous ports:
///   [rocket#652](https://github.com/SergioBenitez/Rocket/issues/652)
///   and [this comment](https://github.com/SergioBenitez/Rocket/issues/652#issuecomment-483515480)
///   on the same issue.
#[derive(Clone, Debug)]
pub struct CgiScript {
    script: ScriptCommand,
    methods: Vec<Method>,
    paths: Vec<String>,
    path_translated: PathBuf,
    rank: isize,
}

impl CgiScript {
    const DEFAULT_RANK: isize = 10;

    // TODO: Add the ability to specify multiple HTTP methods for a single path
    pub fn new<P>(
        command: &str,
        args: &[&str],
        env_vars: &[(&str, &str)],
        paths: &[&str],
        methods: Vec<Method>,
        path_translated: P,
    ) -> Self
    where
        P: AsRef<Path>,
    {
        let command = command.to_string();
        let args: Vec<_> = args.iter().map(|arg| arg.to_string()).collect();
        let env_vars: HashMap<_, _> = env_vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let path_translated = path_translated.as_ref().to_path_buf();

        Self {
            script: ScriptCommand {
                command,
                args,
                env_vars,
            },
            methods,
            paths: paths.iter().map(|p| p.to_string()).collect(),
            path_translated,
            rank: Self::DEFAULT_RANK,
        }
    }

    #[allow(dead_code)]
    pub fn rank(self, rank: isize) -> Self {
        Self { rank, ..self }
    }
}

#[macro_export]
macro_rules! methods {
    ($($method:ident),+) => {{
        use rocket::http::Method::*;
        let mut methods = Vec::new();
        $(
            methods.push($method);
        )+
        methods
    }};
}

impl Handler for CgiScript {
    fn handle<'r>(&self, request: &'r Request, data: Data) -> Outcome<'r> {
        let config = request.guard::<State<Config>>().unwrap();

        let mut cmd = Command::new(&self.script.command);
        cmd.args(&self.script.args).envs(&self.script.env_vars);

        // FIXME: Add more CGI environment variables and ensure the current ones are correct.

        cmd.env("SERVER_SOFTWARE", "rocket")
            .env("SERVER_NAME", config.address.to_string())
            .env("GATEWAY_INTERFACE", "CGI/1.1")
            // FIXME:
            .env("SERVER_PROTOCOL", "HTTP/1.1")
            .env("SERVER_PORT", config.port.to_string());

        cmd.env("REQUEST_METHOD", request.method().as_str())
            .env("QUERY_STRING", request.uri().query().unwrap_or(""))
            .env("REMOTE_HOST", "")
            .env(
                "REMOTE_ADDR",
                request
                    .client_ip()
                    .map(|ip| ip.to_string())
                    .unwrap_or_default(),
            );

        // FIXME: Make sure that paths behave properly when a CgiScript route is mounted
        //        somewhere other than at the root path.
        // FIXME: Handle paths that end in .git
        cmd.env("PATH_INFO", request.uri().path())
            // FIXME: Make sure this does the correct thing.
            .env("PATH_TRANSLATED", &self.path_translated);

        cmd.env("AUTH_TYPE", "").env("REMOTE_USER", "");

        cmd.env("REMOTE_IDENT", "").env(
            "CONTENT_TYPE",
            request
                .content_type()
                .map(|ct| ct.to_string())
                .unwrap_or_default(),
        );

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut process = cmd.spawn().expect("Could not spawn CGI script process");

        if request
            .content_type()
            .map(|ct| ct.is_gzip())
            .unwrap_or(false)
        {
            let mut body = GzDecoder::new(data.open());
            io::copy(&mut body, &mut process.stdin.take().unwrap())
        } else {
            data.stream_to(&mut process.stdin.take().unwrap())
        }
        .expect("Could not copy request body to CGI script stdin");

        let output = process
            .wait_with_output()
            .expect("git http-backend did not execute successfully");
        trace!("{:?}", output.status);
        trace!("{:#?}", output.stdout);
        if !output.stderr.is_empty() {
            trace!("{:#?}", output.stderr);
        }

        parse_cgi_output(request, &output.stdout)
    }
}

impl Into<Vec<Route>> for CgiScript {
    fn into(self) -> Vec<Route> {
        self.paths
            .iter()
            .zip(self.methods.iter())
            .map(|(path, method)| Route::ranked(self.rank, *method, path, self.clone()))
            .collect()
    }
}

#[derive(Clone, Debug)]
struct ScriptCommand {
    command: String,
    args: Vec<String>,
    env_vars: HashMap<String, String>,
}

fn parse_cgi_output<'r>(req: &Request, output: &[u8]) -> Outcome<'r> {
    let header_end_idx = output
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .unwrap();
    let (raw_header, raw_body) = output.split_at(header_end_idx + 1);
    // TODO: Determine if it's only git that wants a leading '\n' in the repsonse body or if this is the standard.
    let raw_body = &raw_body[3..];

    // Copied from https://github.com/tomaka/rouille/blob/master/src/cgi.rs#L142-L158
    // with some modifications.
    let mut headers = Vec::new();
    let mut status_code = 200;
    for header in raw_header.split(|byte| *byte == b'\n') {
        if header.is_empty() {
            break;
        }

        let (raw_name, raw_value) =
            header.split_at(header.iter().position(|byte| *byte == b':').unwrap());
        let last_value_idx = {
            if raw_value.last() == Some(&b'\r') {
                raw_value.len() - 1
            } else {
                raw_value.len()
            }
        };
        let raw_value = &raw_value[2..last_value_idx];

        if raw_name == b"Status" {
            status_code = std::str::from_utf8(&raw_value[0..3])
                .expect("Value of Status contains invalid UTF-8")
                .parse()
                .expect("Status returned by CGI program is invalid");
        } else {
            headers.push((raw_name, raw_value));
        }
    }
    // End of copied section.

    let mut response = Response::new();
    let status =
        Status::from_code(status_code).expect("CGI script returned a nons-standard status code");
    response.set_status(status);
    for (raw_header_name, raw_header_value) in headers {
        response.set_raw_header(
            String::from_utf8(raw_header_name.to_vec())
                .expect("Header name returned from CGI script contains invalid UTF-8"),
            String::from_utf8(raw_header_value.to_vec())
                .expect("Header value returned from CGI script contains invalid UTF-8"),
        );
    }
    response.set_sized_body(io::Cursor::new(raw_body.to_vec()));

    Outcome::from(req, response)
}
