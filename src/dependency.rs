use std::{borrow::Cow, ops::Deref, path::Path};

use url::Url;

use crate::common::{self, Error::{self, ParseError}, TemplateKind, UrlError};

const URL_RAYLIB_DEFAULT: &'static str = "https://github.com/raysan5/raylib/releases/download/5.5/raylib-5.5_win64_mingw-w64.zip"; 

fn parse_dependency_url(url: &str) -> Result<String, UrlError> {
    let url = Url::parse(url).map_err(|e| UrlError::InvalidUrl(e))?;

    if url.scheme() != "https" {
        return Err(UrlError::NotHttps);
    }

    if url.host_str() != Some("github.com") {
        return Err(UrlError::NotGithub);
    }

    let path = url.path(); 

    if !path.ends_with(".zip") {
        return Err(UrlError::NotZip);
    }

    let file_name = path
        .split("/")
        .last()
        .ok_or(UrlError::NoFileName)
        .and_then(|p| p.strip_suffix(".zip").ok_or(UrlError::NoFileName))?;

    // TODO: check ../.. relative paths and % signs in filenames

    Ok(file_name.to_owned())
}

pub(crate) struct Dependency<'a> {
    pub(crate) name: Cow<'a, str>,
    pub(crate) url: Cow<'a, str>,
    pub(crate) no_root: bool,
}

impl<'a> Dependency<'a> {
    pub(crate) fn new(name: Cow<'a, str>, url: Cow<'a, str>) -> Self {
        Dependency { 
            name, 
            url, 
            no_root: false 
        }
    }

    pub(crate) fn no_root(mut self, no_root: bool) -> Self {
        self.no_root = no_root;
        self
    }
}

#[derive(Default)]
pub(crate) struct Dependencies {
    inner: Vec<Dependency<'static>>
}

// impl Default for Dependencies {

// }

impl Dependencies {
    pub(crate) fn from_template(kind: TemplateKind) -> Option<Self> {
        match kind  {
            TemplateKind::Main => None,
            TemplateKind::Raylib => Some(Self {
                inner: vec![Dependency::new("raylib".into(), URL_RAYLIB_DEFAULT.into())]
            })
        }
    }

    pub(crate) fn empty() -> Self {
        Self { inner: vec![] }
    }

    pub(crate) fn from_file_if_exists<P: AsRef<Path>>(path: &P) -> Result<Option<Self>, Error> {
        common::file_read_if_exists(path).map_err(|e| e.into()).and_then(|contents| {
            let contents = match contents {
                Some(c) => c,
                None => return Ok(None),
            };

            let mut result = Dependencies { inner: vec![] };

            for l in contents.lines() {
                let mut l = l.split_whitespace();
                let name = l.next().ok_or_else(|| ParseError("Deps file has no dependency name".into()))?;
                let no_root = match l.next().ok_or_else(|| ParseError("Deps file has no dependency name".into()))? {
                    "nr" => true,
                    "r" => false,
                    _ => return Err(ParseError("Deps file has an invalid no_root flag".into())),  
                };
                let url = l.next().ok_or_else(|| ParseError("Deps file has no url".into()))?; 
                result.add_dependency(Some(name.into()), url, no_root)?;
            }

            Ok(Some(result))
        })
    }

    pub(crate) fn add_dependency(&mut self, name: Option<String>, url: &str, no_root: bool) -> Result<(), UrlError> {
        let name_from_url = parse_dependency_url(url)?;

        self.inner.push(Dependency::new(
            name.or(Some(name_from_url)).unwrap().into(), 
            url.to_owned().into()
        ).no_root(no_root));

        Ok(())
    }

    pub(crate) fn remove_dependencies(&mut self, indices: &[usize]) {
        indices.iter().for_each(|idx| _ = self.inner.remove(*idx));
    }
}

impl Deref for Dependencies {
    type Target = [Dependency<'static>];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}