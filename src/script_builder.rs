use std::{fmt::Debug};
use crate::dependency::{Dependency};

#[derive(Default)]
pub(crate) enum ScriptKind {
    #[default]
    Build,
    Run,
    Setup
}

#[derive(Default)]
pub(crate) struct ScriptBuilder<'a> {
    kind: ScriptKind,
    cflags: Option<&'a str>,
    target_name: Option<&'a str>,
    src_file_names: Option<&'a str>,
    include_flags: Option<&'a str>,
    linker_flags: Option<&'a str>,
    dependencies: Option<&'a [Dependency<'a>]>,
    need_dependencies: bool
}

pub(crate) enum ScriptError {
    UnspecifiedField(&'static str),
}

impl Debug for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptError::UnspecifiedField(field) => write!(f, "UnspecifiedField(\"{}\")", field),
        }
    }
}

impl<'a> ScriptBuilder<'a> {
    pub(crate) fn new(kind: ScriptKind) -> Self {
        ScriptBuilder { kind: kind, ..Default::default() }
    }

    pub(crate) fn cflags(mut self, flags: &'a str) -> Self {
        self.cflags = Some(flags);
        self
    }

    pub(crate) fn target_name(mut self, target_name: &'a str) -> Self {
        self.target_name = Some(target_name);
        self
    }

    pub(crate) fn src_file_names(mut self, file_names: &'a str) -> Self {
        self.src_file_names = Some(file_names);
        self
    }

    pub(crate) fn include_flags(mut self, include_flags: &'a str) -> Self {
        self.include_flags = Some(include_flags);
        self.need_dependencies = true;
        self
    }

    pub(crate) fn linker_flags(mut self, linker_flags: &'a str) -> Self {
        self.linker_flags = Some(linker_flags);
        self.need_dependencies = true;
        self
    }

    pub(crate) fn dep_urls(mut self, dependencies: &'a [Dependency<'a>]) -> Self {
        self.dependencies = Some(dependencies);
        self
    }

    pub(crate) fn build(self) -> Result<String, ScriptError> {
        Ok(match self.kind {
            ScriptKind::Build => {
                let (include_flags, linker_flags) = if self.need_dependencies {
                    (
                        self.include_flags.ok_or(ScriptError::UnspecifiedField("include_flags"))?, 
                        self.linker_flags.ok_or(ScriptError::UnspecifiedField("linker_flags"))?
                    )
                } else { ("", "") };

                let setup_clause = if self.need_dependencies {
                    "if not exist lib (\
                    \ncall .\\setup.bat\
                    \n)"
                } else { "" };

                format!(
                    "@echo off\
                    \nset TARGET_NAME={}.exe\
                    \nset SRC_FILE_NAMES={}\
                    \nset ROOT_FOLDER=%~dp0\
                    \nset CFLAGS={}\
                    \nset INCLUDES={}\
                    \nset LINKER={}\
                    \n{}\
                    \nsetlocal enabledelayedexpansion\
                    \nset SRC_PATHS=\
                    \nfor %%i in (%SRC_FILE_NAMES%) do (set SRC_PATHS=!SRC_PATHS! %ROOT_FOLDER%src\\%%i)\
                    \npushd %ROOT_FOLDER%\
                    \nif not exist bin mkdir bin\
                    \n@echo on\
                    \ngcc %INCLUDES% %LINKER% %SRC_PATHS% -o bin/%TARGET_NAME% %CFLAGS%\
                    \n@echo off\
                    \nif %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%\
                    \necho build success.\
                    \nendlocal",
                    self.target_name.ok_or(ScriptError::UnspecifiedField("target_name"))?,
                    self.src_file_names.ok_or(ScriptError::UnspecifiedField("src_file_names"))?,
                    self.cflags.ok_or(ScriptError::UnspecifiedField("cflags"))?,
                    include_flags,
                    linker_flags,
                    setup_clause
                )
            },
            ScriptKind::Setup => {
                let urls = self
                    .dependencies
                    .ok_or(ScriptError::UnspecifiedField("dep_urls"))?
                    .iter()
                    .map(|Dependency { name, url, no_root }| {
                        let strip_flag = if *no_root { 0 } else { 1 };

                        // TODO: check error level
                        format!(
                            "curl -fsSL -o {}.zip {}\
                            \nif not exist {} mkdir {}\
                            \ntar -xf {}.zip --strip-components={} -C {}\
                            \nmove {} lib\\{}\
                            \ndel {}.zip",
                            name, url.as_ref(), name, name, name, strip_flag, name, name, name, name
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                format!(
                    "if not exist lib mkdir lib\
                    \n{}",
                    urls
                )
            },
            ScriptKind::Run => {
                format!(
                    "@echo off\
                    \nset TARGET_NAME={}.exe\
                    \nset ROOT_FOLDER=%~dp0\
                    \nset CLI_ARGS=%*\
                    \npushd %ROOT_FOLDER%\
                    \ncall ./build.bat\
                    \nif %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%\
                    \necho:\
                    \npushd bin\
                    \n%TARGET_NAME% %CLI_ARGS%\
                    \npopd\
                    \npopd",
                    self.target_name.ok_or(ScriptError::UnspecifiedField("target_name"))?
                )
            }
        })
    } 
}