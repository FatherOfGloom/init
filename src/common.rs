use std::{fmt::{self, Display}, fs, path::{Path, PathBuf}};

#[derive(Clone, Copy)]
pub(crate) enum TemplateKind {
    Main,
    Raylib,
}

impl TemplateKind {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            TemplateKind::Main => "main",
            TemplateKind::Raylib => "rl",
        }
    }
}

pub(crate) fn file_read_if_exists<P: AsRef<Path>>(path: P) -> Result<Option<String>, std::io::Error> {
    Ok(if path.as_ref().try_exists()? {
        Some(fs::read_to_string(path)?)
    } else { 
        None 
    })
}

#[derive(Debug)]
pub(crate) enum UrlError {
    InvalidUrl(url::ParseError),
    NotGithub,
    NotHttps,
    NoFileName,
    NotZip,
}

impl Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlError::InvalidUrl(e) => write!(f, "invalid url: {}", e),
            UrlError::NotGithub => write!(f, "url host is not github.com"),
            UrlError::NotHttps => write!(f, "url scheme is not https"),
            UrlError::NoFileName => write!(f, "could not determine file name from url"),
            UrlError::NotZip => write!(f, "url does not point to a .zip file"),
        }
    }
}
    
#[derive(Debug)]
pub(crate) enum Error {
    Io(String),
    AlreadyExists(PathBuf),
    NeedInit(PathBuf),
    ParseError(String),
    CFlagNotUnique(String),
    CFlagNotFound(String),
    UrlError(UrlError),
    DepNotFoundByName(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::AlreadyExists(path) => write!(f, "Init failed: {} already exists.", path.display()),
            Error::NeedInit(path) => write!(f, 
                "Failed: unable to find .init directory at path: {}. You need to run init to setup a project.", 
                path.display()
            ),
            Error::ParseError(e) => write!(f, "Parsing cache file failed: {}.", e),
            Error::CFlagNotUnique(flag) => write!(f, "Provided cflag '{}' already exists", flag),
            Error::CFlagNotFound(flag) => write!(f, "Provided cflag '{}' doesn't exists", flag),
            Error::UrlError(e) => write!(f, "Url parse error: {}", e),
            Error::DepNotFoundByName(name) => write!(f, "Dependency not found: {}", name),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value.to_string())
    }
}

impl From<&std::io::Error> for Error {
    fn from(value: &std::io::Error) -> Self {
        Error::Io(value.to_string())
    }
}

impl From<UrlError> for Error {
    fn from(value: UrlError) -> Self {
        Self::UrlError(value)
    }
}

#[macro_export]
macro_rules! leak {
    () => {
        ($($val:expr),* $(,)?) => {
            std::mem::forget($val);
        }
    };
}