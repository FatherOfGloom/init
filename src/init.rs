use std::{borrow::Cow, cell::{LazyCell, Ref, RefCell}, fmt::Display, fs, path::PathBuf, process::Command};

pub(crate) struct ConfigCache<'a> {
    src_file_names: Cow<'a, str>,
    cflags: Option<Cow<'a, str>>,
    target_executable_name: Cow<'a, str>,
    template_kind: TemplateKind,
    build_script_cache: RefCell<Option<String>>,
    run_script_cache: RefCell<Option<String>>,
}

impl ConfigCache<'_> {
    fn template(&self) -> &'static str {
        match self.template_kind {
            TemplateKind::Main => TEMPLATE_C_DEFAULT,
            _ => todo!(),
        }
    }

    fn cflags(&self) -> &str {
        self.cflags.as_ref().map(|c| c.as_ref()).unwrap_or(TEMPLATE_CFLAGS_DEFAULT)
    }

    fn cache_serialize(&self) -> String {
        format!(
            "template {}\n",
            self.template_kind.as_str()
        )
    }

    fn build_win(&self) -> Ref<'_, str> {
       if self.build_script_cache.borrow().is_none() {
            *self.build_script_cache.borrow_mut() = Some(format!(
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
                self.target_executable_name,
                self.src_file_names,
                self.cflags()
            ));
        }

        Ref::map(self.build_script_cache.borrow(), |opt| opt.as_ref().map(|s| s.as_str()).unwrap())
    }

    fn run(&self) -> Ref<'_, str> {
       if self.run_script_cache.borrow().is_none() {
            *self.run_script_cache.borrow_mut() = Some(format!(
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
                self.target_executable_name
            ));
        }

        Ref::map(self.run_script_cache.borrow(), |opt| opt.as_ref().map(|s| s.as_str()).unwrap())
    }

    fn gitignore(&self) -> &str {
        TEMPLATE_GITIGNORE_DEFAULT
    }

    fn clang_format(&self) -> &str {
        TEMPLATE_CLANG_FORMAT_DEFAULT
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    Io(String),
    AlreadyExists(PathBuf),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::AlreadyExists(path) => write!(f, "Init failed: {} already exists", path.display()),
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

pub(crate) struct Init<'a> {
    root_folder_display: LazyCell<Result<PathBuf, std::io::Error>>,
    root_colder_canonical: LazyCell<Result<PathBuf, std::io::Error>>,
    config: RefCell<Option<ConfigCache<'a>>>,
}

enum TemplateKind {
    Main,
    Raylib,
}

impl TemplateKind {
    fn as_str(&self) -> &str {
        match self {
            TemplateKind::Main => "main",
            TemplateKind::Raylib => "rl",
        }
    }
}

pub(crate) struct InitOptions {
    target_executable_name: Option<String>,
    template_kind: TemplateKind,
}

impl InitOptions {
    pub(crate) fn raylib(exe_name: Option<String>) -> Self {
        InitOptions { target_executable_name: exe_name, template_kind: TemplateKind::Raylib }
    }
}

impl Default for InitOptions {
    fn default() -> Self {
        Self { 
            target_executable_name: None,
            template_kind: TemplateKind::Main,
        }
    }
}

const TEMPLATE_C_DEFAULT: &'static str = include_str!("templates/main_default.c"); 
const TEMPLATE_CFLAGS_DEFAULT: &'static str = "-Wall -Wextra -Wpedantic -fanalyzer -std=c11";
const TEMPLATE_FILE_NAMES_DEFAULT: &'static str = "main.c";
const TEMPLATE_CLANG_FORMAT_DEFAULT: &'static str = include_str!("templates/clang_format_default");
const TEMPLATE_GITIGNORE_DEFAULT: &'static str = include_str!("templates/gitignore_default");

impl Init<'_> {
    pub(crate) fn new() -> Self {
        Init { 
            root_folder_display: LazyCell::new(|| { std::path::absolute(".") }),
            root_colder_canonical: LazyCell::new(|| { std::fs::canonicalize(".") }),
            config: RefCell::new(None),
        }
    }

    fn config(&self, init_options: Option<InitOptions>) -> Result<Ref<'_, ConfigCache<'_>>, Error> {
        if self.config.borrow().is_none() {
            let root_path_canonical = self.root_colder_canonical.as_ref()?;
            let init_options = init_options.unwrap_or_default();

            let exe = init_options
                .target_executable_name
                .map(|n| Cow::Owned(n))
                .unwrap_or_else(|| Cow::Owned(root_path_canonical
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string()
                ));

            *self.config.borrow_mut() = Some(
                ConfigCache { 
                    src_file_names: Cow::Borrowed(TEMPLATE_FILE_NAMES_DEFAULT),
                    cflags: Some(Cow::Borrowed(TEMPLATE_CFLAGS_DEFAULT)),
                    target_executable_name: exe,
                    template_kind: init_options.template_kind,
                    build_script_cache: RefCell::new(None),
                    run_script_cache: RefCell::new(None),
                }
            ); 
        }

        Ok(Ref::map(self.config.borrow(), |c| c.as_ref().unwrap()))
    }

    pub(crate) fn init(&mut self, options: Option<InitOptions>) -> Result<(), Error> {
        let root_path_canonical = self.root_colder_canonical.as_ref()?;
        let root_folder_display = self.root_folder_display.as_ref()?;

        eprintln!("Initializing a C project in {}...", root_folder_display.display());

        let init_path = root_path_canonical.join(".init");

        if init_path.exists() {
            return Err(Error::AlreadyExists(init_path));
        }

        let src_path = root_path_canonical.join("src");

        if src_path.exists() {
            return Err(Error::AlreadyExists(src_path));
        }

        fs::create_dir(&init_path)?;
        fs::create_dir(&src_path)?;

        let config = self.config(options)?;

        fs::write(src_path.join("main.c"), config.template())?;

        if cfg!(windows) {
            fs::write(init_path.join("cflagswin"), config.cflags())?;
            fs::write(init_path.join("cache"), config.cache_serialize())?;
            // TODO: deps
            // TODO: linker
            fs::write(root_path_canonical.join("build.bat"), &*config.build_win())?;
            fs::write(root_path_canonical.join("run.bat"), &*config.run())?;
            fs::write(root_path_canonical.join(".gitignore"), &*config.gitignore())?;
            fs::write(root_path_canonical.join(".clang-format"), &*config.clang_format())?;

            match Command::new("git").arg("init").output() {
                Err(e) => eprintln!("Failed to execute git init: {}", e.to_string()),
                Ok(out) => {
                    if out.status.success() { 
                        eprintln!("Successfully initialized a git repository"); 
                    } else {
                        eprintln!("Git wasn't able to init a repo: {}", String::from_utf8_lossy(&out.stderr));
                    }
                }
            } 

        } else {
            todo!("For now only supports windows.");
        }

        Ok(())
    }
}