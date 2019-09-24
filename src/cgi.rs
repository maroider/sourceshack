use std::{
    collections::HashMap,
    fmt,
    io::{self, Read},
    process::{Command, Stdio},
};

pub struct CgiScript<'a> {
    command: &'a str,
    args: &'a [&'a str],
    env_vars: &'a [(&'a str, &'a str)],
    server_software: Option<&'a str>,
    server_name: Option<&'a str>,
    server_port: Option<&'a str>,
    request_method: Option<&'a str>,
    query_string: Option<&'a str>,
    remote_host: Option<&'a str>,
    remote_addr: Option<&'a str>,
    path_info: Option<&'a str>,
    path_translated: Option<&'a str>,
    auth_type: Option<&'a str>,
    remote_user: Option<&'a str>,
    remote_ident: Option<&'a str>,
    content_type: Option<&'a str>,
}

macro_rules! builder_property {
    ($property:ident, $ty:ty) => {
        pub fn $property(self, $property: $ty) -> Self {
            Self {
                $property: Some($property),
                ..self
            }
        }
    };
}

impl<'a> CgiScript<'a> {
    pub fn new(command: &'a str, args: &'a [&'a str], env_vars: &'a [(&'a str, &'a str)]) -> Self {
        Self {
            command,
            args,
            env_vars,
            server_software: None,
            server_name: None,
            server_port: None,
            request_method: None,
            query_string: None,
            remote_host: None,
            remote_addr: None,
            path_info: None,
            path_translated: None,
            auth_type: None,
            remote_user: None,
            remote_ident: None,
            content_type: None,
        }
    }

    builder_property!(server_software, &'a str);
    builder_property!(server_name, &'a str);
    builder_property!(server_port, &'a str);
    builder_property!(request_method, &'a str);
    builder_property!(query_string, &'a str);
    builder_property!(remote_host, &'a str);
    builder_property!(remote_addr, &'a str);
    builder_property!(path_info, &'a str);
    builder_property!(path_translated, &'a str);
    builder_property!(auth_type, &'a str);
    builder_property!(remote_user, &'a str);
    builder_property!(remote_ident, &'a str);
    builder_property!(content_type, &'a str);

    pub fn run<R: Read>(self, data: R) -> Result<CgiResponse, CgiScriptError> {
        let mut cmd = Command::new(&self.command);
        cmd.args(self.args).envs(
            self.env_vars
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string())),
        );

        // FIXME: Add more CGI environment variables and ensure the current ones are correct.

        cmd.env("SERVER_SOFTWARE", self.server_software.unwrap_or(""))
            .env("SERVER_NAME", self.server_name.unwrap_or(""))
            .env("GATEWAY_INTERFACE", "CGI/1.1")
            // FIXME:
            .env("SERVER_PROTOCOL", "HTTP/1.1")
            .env("SERVER_PORT", self.server_port.unwrap_or(""));

        cmd.env("REQUEST_METHOD", self.request_method.unwrap_or(""))
            .env("QUERY_STRING", self.query_string.unwrap_or(""))
            .env("REMOTE_HOST", self.remote_host.unwrap_or(""))
            .env("REMOTE_ADDR", self.remote_addr.unwrap_or(""));

        // FIXME: Make sure that paths behave properly when a CgiScript route is mounted
        //        somewhere other than at the root path.
        // FIXME: Handle paths that end in .git
        cmd.env("PATH_INFO", self.path_info.unwrap_or(""))
            // FIXME: Make sure this does the correct thing.
            .env("PATH_TRANSLATED", self.path_translated.unwrap_or(""));

        cmd.env("AUTH_TYPE", self.auth_type.unwrap_or(""))
            .env("REMOTE_USER", self.remote_user.unwrap_or(""));

        cmd.env("REMOTE_IDENT", self.remote_ident.unwrap_or(""))
            .env("CONTENT_TYPE", self.content_type.unwrap_or(""));

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut process = cmd.spawn()?;
        io::copy(&mut data, &mut process.stdin.take().unwrap())?;
        let output = process.wait_with_output()?;

        Ok(parse_cgi_output(&output.stdout)?)
    }
}

#[derive(Debug)]
pub enum CgiScriptError {
    Io(io::Error),
    ParseOutput(ParseCgiOutputError),
}

impl fmt::Display for CgiScriptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error while running CGI script: {}", err),
            Self::ParseOutput(err) => write!(f, "Error while parsing CGI script output: {}", err),
        }
    }
}

impl std::error::Error for CgiScriptError {}

impl From<io::Error> for CgiScriptError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<ParseCgiOutputError> for CgiScriptError {
    fn from(err: ParseCgiOutputError) -> Self {
        Self::ParseOutput(err)
    }
}

fn parse_cgi_output<'r>(output: &[u8]) -> Result<CgiResponse, ParseCgiOutputError> {
    let header_end_idx = output
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .ok_or(ParseCgiOutputError::NoEndOfHeader)?;
    let (raw_header, raw_body) = output.split_at(header_end_idx + 1);
    // TODO: Determine if it's only git that wants a leading '\n' in the repsonse body or if this is the standard.
    let raw_body = &raw_body[3..];

    // Copied from https://github.com/tomaka/rouille/blob/master/src/cgi.rs#L142-L158
    // with some modifications.
    let mut headers_vec = Vec::new();
    let mut status_code = 200;
    for header in raw_header.split(|byte| *byte == b'\n') {
        if header.is_empty() {
            break;
        }

        let (raw_name, raw_value) = header.split_at(
            header
                .iter()
                .position(|byte| *byte == b':')
                .ok_or(ParseCgiOutputError::NoHeaderValue)?,
        );
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
                .map_err(|_| ParseCgiOutputError::InvalidUtf8InStatus)?
                .parse()
                .map_err(|_| ParseCgiOutputError::InvalidStatus)?;
        } else {
            headers_vec.push((raw_name, raw_value));
        }
    }
    // End of copied section.

    let mut headers = HashMap::with_capacity(headers_vec.len());
    for (raw_header_name, raw_header_value) in headers_vec {
        headers.insert(
            String::from_utf8(raw_header_name.to_vec())
                .map_err(|_| ParseCgiOutputError::InvalidUtf8InHeaderName)?,
            String::from_utf8(raw_header_value.to_vec())
                .map_err(|_| ParseCgiOutputError::InvalidUtf8InHeaderValue)?,
        );
    }
    let body = raw_body.to_vec();

    Ok(CgiResponse {
        status_code,
        headers,
        body,
    })
}

#[derive(Debug)]
pub enum ParseCgiOutputError {
    NoEndOfHeader,
    NoHeaderValue,
    InvalidUtf8InStatus,
    InvalidStatus,
    InvalidUtf8InHeaderName,
    InvalidUtf8InHeaderValue,
}

impl fmt::Display for ParseCgiOutputError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NoEndOfHeader => write!(
                f,
                r"Could not find end of header: No '\r\n\r\n' sequence was found"
            ),
            Self::NoHeaderValue => write!(f, "Could not find header value: No ': ' delimiter"),
            Self::InvalidUtf8InStatus => write!(f, "Status code contains invalid UTF-8"),
            Self::InvalidStatus => write!(f, "Invalid status code"),
            Self::InvalidUtf8InHeaderName => write!(f, "Invalid UTF-8 in header name"),
            Self::InvalidUtf8InHeaderValue => write!(f, "Invalid UTF-8 in header value"),
        }
    }
}

impl std::error::Error for ParseCgiOutputError {}

pub struct CgiResponse {
    status_code: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}
