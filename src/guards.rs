use std::{ffi::OsStr, fmt, path::Path};

use rocket::{
    http::RawStr,
    request::{FromFormValue, FromParam},
};

macro_rules! string_wrapper_impls {
    ($string_wrapper:ident $( < $( $lt:lifetime ),* $( , )? > )? , $field:ident ) => {
        impl<'a> $string_wrapper<'a> {
            #[allow(dead_code)]
            pub fn as_str(&self) -> &str {
                self.$field.as_ref()
            }

            fn as_bytes(&self) -> &[u8] {
                self.$field.as_bytes()
            }
        }

        impl<'a> AsRef<str> for $string_wrapper<'a> {
            fn as_ref(&self) -> &str {
                self.$field.as_ref()
            }
        }

        impl<'a> AsRef<[u8]> for $string_wrapper<'a> {
            fn as_ref(&self) -> &[u8] {
                self.as_bytes()
            }
        }

        impl<'a> AsRef<OsStr> for $string_wrapper<'a> {
            fn as_ref(&self) -> &OsStr {
                self.$field.as_ref()
            }
        }

        impl<'a> AsRef<Path> for $string_wrapper<'a> {
            fn as_ref(&self) -> &Path {
                self.$field.as_ref()
            }
        }

        impl<'a> fmt::Display for $string_wrapper<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.$field.fmt(f)
            }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserNameGuard<'a> {
    name: AaudStr<'a>,
}

impl<'a> FromParam<'a> for UserNameGuard<'a> {
    type Error = &'a RawStr;

    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> {
        let name = param.as_str();
        if !(name.starts_with("~")) {
            return Err(param);
        }
        let name = &name[1..];
        let name = AaudStr::from_param(RawStr::from_str(name))?;
        Ok(Self { name })
    }
}

/// "ASCCI Alphanumeric + Underscore + Dash"-string
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AaudStr<'a> {
    inner: &'a str,
}

impl<'a> AaudStr<'a> {
    pub fn new(str: &'a str) -> Option<Self> {
        if str
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ['_', '-'].contains(&c))
        {
            Some(Self { inner: str })
        } else {
            None
        }
    }

    pub fn is_valid(str: &'a str) -> bool {
        Self::new(str).is_some()
    }
}

impl<'a> FromParam<'a> for AaudStr<'a> {
    type Error = &'a RawStr;

    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> {
        let string = param.as_str();
        if string.is_empty() {
            return Err(param);
        }
        if let Some(str) = Self::new(string) {
            Ok(str)
        } else {
            Err(param)
        }
    }
}

impl<'a> FromFormValue<'a> for AaudStr<'a> {
    type Error = &'a RawStr;

    fn from_form_value(form_value: &'a RawStr) -> Result<Self, Self::Error> {
        let string = form_value.as_str();
        if let Some(str) = Self::new(string) {
            Ok(str)
        } else {
            Err(form_value)
        }
    }

    fn default() -> Option<Self> {
        None
    }
}

string_wrapper_impls!(UserNameGuard<'a>, name);
string_wrapper_impls!(AaudStr<'a>, inner);
