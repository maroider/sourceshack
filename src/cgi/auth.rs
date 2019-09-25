use std::str::FromStr;

#[derive(Clone, Copy, Debug)]
pub enum Auth {
    Basic,
    Digest,
}

impl Auth {
    pub fn as_str(self) -> &'static str {
        match self {
            Auth::Basic => "Basic",
            Auth::Digest => "Digest",
        }
    }
}

impl FromStr for Auth {
    type Err = AuthParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Basic" => Ok(Self::Basic),
            "Digest" => Ok(Self::Digest),
            _ => Err(AuthParseError { _priv: () }),
        }
    }
}

pub struct AuthParseError {
    _priv: (),
}

impl From<Auth> for &'static str {
    fn from(auth: Auth) -> Self {
        auth.as_str()
    }
}
