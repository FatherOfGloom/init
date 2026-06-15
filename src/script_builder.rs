#[derive(Default)]
pub(crate) enum ScriptKind {
    #[default]
    Build,
    Run,
}

#[derive(Default)]
pub(crate) struct ScriptBuilder<'a> {
    kind: ScriptKind,
    cflags: Option<&'a str>,
    target_name: Option<&'a str>,
    src_file_names: Option<&'a str>
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

    pub(crate) fn build(self) -> String {
        match self.kind {
            ScriptKind::Build => {
                format!(
                    "@echo off\
                    \nset TARGET_NAME={}.exe\
                    \nset SRC_FILE_NAMES={}\
                    \nset ROOT_FOLDER=%~dp0\
                    \nset CFLAGS={}\
                    \nsetlocal enabledelayedexpansion\
                    \nset SRC_PATHS=\
                    \nfor %%i in (%SRC_FILE_NAMES%) do (set SRC_PATHS=!SRC_PATHS! %ROOT_FOLDER%src\\%%i)\
                    \npushd %ROOT_FOLDER%\
                    \nif not exist bin mkdir bin\
                    \n@echo on\
                    \ngcc %SRC_PATHS% -o bin/%TARGET_NAME% %CFLAGS%\
                    \n@echo off\
                    \nif %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%\
                    \necho build success.\
                    \nendlocal",
                    self.target_name.unwrap(),
                    self.src_file_names.unwrap(),
                    self.cflags.unwrap()
                )
            }
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
                    self.target_name.unwrap()
                )
            }
        }
    } 
}