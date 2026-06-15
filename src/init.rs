use std::{borrow::Cow, cell::{LazyCell, Ref, RefCell}, collections::HashSet, fmt::Display, fs, path::{Path, PathBuf}, process::Command};

use crate::{init::Error::{CFlagNotUnique, CFlagNotFound, ParseError}, script_builder::{ScriptBuilder, ScriptKind}};

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
            TemplateKind::Raylib => todo!(),
        }
    }

    fn cflags(&self) -> &str {
        self.cflags.as_ref().map(|c| c.as_ref()).unwrap_or(TEMPLATE_CFLAGS_DEFAULT)
    }

    fn cache(&self) -> String {
        format!(
            "template {}\n",
            self.template_kind.as_str()
        )
    }

    fn build(&self) -> Ref<'_, str> {
       if self.build_script_cache.borrow().is_none() {
            let script = ScriptBuilder::new(ScriptKind::Build)
                .cflags(self.cflags())
                .src_file_names(&self.src_file_names)
                .target_name(&self.target_executable_name)
                .build()
                .unwrap();

            *self.build_script_cache.borrow_mut() = Some(script);
        }

        Ref::map(self.build_script_cache.borrow(), |opt| opt.as_ref().map(|s| s.as_str()).unwrap())
    }

    fn run(&self) -> Ref<'_, str> {
       if self.run_script_cache.borrow().is_none() {
            let script = ScriptBuilder::new(ScriptKind::Run)
                .target_name(&self.target_executable_name)
                .build()
                .unwrap();

            *self.run_script_cache.borrow_mut() = Some(script);
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
    NeedInit(PathBuf),
    ParseError(String),
    CFlagNotUnique(String),
    CFlagNotFound(String),
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
            Error::CFlagNotFound(flag) => write!(f, "Provided cflag '{}' doesn't exists", flag)
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
    root_folder_canonical: LazyCell<Result<PathBuf, std::io::Error>>,
    config: RefCell<Option<ConfigCache<'a>>>,
    init_path_cache: RefCell<Option<PathBuf>>,
}

pub(crate) enum TemplateKind {
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

pub(crate) enum InitOptions {
    FromCommand {
        target_executable_name: Option<String>,
        template_kind: TemplateKind,
    },
    FromFile
}

impl InitOptions {
    pub(crate) fn raylib(exe_name: Option<String>) -> Self {
        InitOptions::FromCommand { 
            target_executable_name: exe_name, 
            template_kind: TemplateKind::Raylib, 
        }
    }

    pub(crate) fn from_file() -> Self {
        InitOptions::FromFile
    }
}

impl Default for InitOptions {
    fn default() -> Self {
        Self::FromCommand {
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
            root_folder_canonical: LazyCell::new(|| { std::fs::canonicalize(".") }),
            config: RefCell::new(None),
            init_path_cache: RefCell::new(None)
        }
    }

    fn init_path(&self) -> Result<Ref<'_, Path>, Error> {
        if self.init_path_cache.borrow().is_none() {
            *self.init_path_cache.borrow_mut() = Some(self.root_folder_canonical.as_ref()?.join(".init"));
        }

        Ok(Ref::map(self.init_path_cache.borrow(), |opt| opt.as_ref().map(|p| p.as_path()).unwrap()))
    }

    fn init_path_take(&self) -> Result<PathBuf, Error> {
        if self.init_path_cache.borrow().is_none() {
            *self.init_path_cache.borrow_mut() = Some(self.root_folder_canonical.as_ref()?.join(".init"));
        }

        Ok(self.init_path_cache.take().unwrap())
    }

    fn config(&self, init_options: Option<InitOptions>) -> Result<Ref<'_, ConfigCache<'_>>, Error> {
        if self.config.borrow().is_none() {
            let root_path_canonical = self.root_folder_canonical.as_ref()?;
            let init_options = init_options.unwrap_or_default();

            let (cflags, exe, kind) = match init_options {
                InitOptions::FromCommand { target_executable_name, template_kind } => { 
                    let exe = target_executable_name
                        .map(|n| Cow::Owned(n));
                    (Cow::Borrowed(TEMPLATE_CFLAGS_DEFAULT), exe, template_kind)
                },
                InitOptions::FromFile => {
                    // TODO: cache_load_from_file function
                    let init_path = self.init_path()?;
                    let cflags = fs::read_to_string(init_path.join("cflagswin"))?;
                    let kind = match fs::read_to_string(init_path.join("cache"))?
                        .split_whitespace()
                        .nth(1)
                        .ok_or_else(|| ParseError("Unable to locate 'template' parameter".into()))?
                        .to_uppercase()
                        .as_str() {
                        "MAIN" => Ok(TemplateKind::Main),
                        "RL" => Ok(TemplateKind::Raylib),
                        _ => Err(ParseError("Unrecognised 'template' parameter".into()))
                    }?;

                    (Cow::Owned(cflags), None, kind)
                }
            };

            let exe = exe.unwrap_or_else(|| Cow::Owned(root_path_canonical
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
            ));

            *self.config.borrow_mut() = Some(ConfigCache { 
                src_file_names: Cow::Borrowed(TEMPLATE_FILE_NAMES_DEFAULT),
                cflags: Some(cflags),
                target_executable_name: exe,
                template_kind: kind,
                build_script_cache: RefCell::new(None),
                run_script_cache: RefCell::new(None),
            }); 
        }

        Ok(Ref::map(self.config.borrow(), |c| c.as_ref().unwrap()))
    }

    pub(crate) fn init(&mut self, options: Option<InitOptions>) -> Result<(), Error> {
        let root_folder_canonical = self.root_folder_canonical.as_ref()?;
        let root_folder_display = self.root_folder_display.as_ref()?;

        eprintln!("Initializing a C project in {}...", root_folder_display.display());

        let init_path = self.init_path()?;

        if init_path.exists() {
            return Err(Error::AlreadyExists(self.init_path_take()?));
        }

        let src_path = root_folder_canonical.join("src");

        if src_path.exists() {
            return Err(Error::AlreadyExists(src_path));
        }

        fs::create_dir(&*init_path)?;
        fs::create_dir(&src_path)?;

        let config = self.config(options)?;

        fs::write(src_path.join("main.c"), config.template())?;

        if cfg!(windows) {
            fs::write(init_path.join("cflagswin"), config.cflags())?;
            fs::write(init_path.join("cache"), config.cache())?;
            // TODO: deps
            // TODO: linker
            fs::write(root_folder_canonical.join("build.bat"), &*config.build())?;
            fs::write(root_folder_canonical.join("run.bat"), &*config.run())?;
            fs::write(root_folder_canonical.join(".gitignore"), &*config.gitignore())?;
            fs::write(root_folder_canonical.join(".clang-format"), &*config.clang_format())?;

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

    pub(crate) fn add_cflags(&self, cflags: &str) -> Result<(), Error> {
        let init_path = self.init_path()?;

        if !init_path.exists() {
            return Err(Error::NeedInit(self.init_path_take()?));
        }

        let config = self.config(Some(InitOptions::from_file()))?;
        let old_flags = config.cflags.as_ref().unwrap();
        let old_flags_hs = old_flags.split_whitespace().collect::<HashSet<&str>>();

        let cflags = cflags.split_whitespace().map(|f| {
            if !f.starts_with("-") { format!("-{}", f.trim())} else { f.trim().to_string() }
        }).collect::<Vec<String>>();
        
        if let Some(f) = cflags.iter().find(|f| old_flags_hs.contains(f.as_str())) {
            return Err(CFlagNotUnique(f.into())); 
        }

        let new_flags = format!("{} {}", old_flags, cflags.join(" "));
        println!("New cflags {}", new_flags);

        let build = ScriptBuilder::new(ScriptKind::Build)
            .cflags(&new_flags)
            .src_file_names(&config.src_file_names)
            .target_name(&config.target_executable_name)
            .build()
            .unwrap();

        fs::write(init_path.join("cflagswin"), new_flags)?;
        fs::write(self.root_folder_canonical.as_ref()?.join("build.bat"), build)?;
        
        Ok(())
    }

    pub(crate) fn remove_cflags(&self, cflags: &str) -> Result<(), Error> {
        let init_path = self.init_path()?;

        if !init_path.exists() {
            return Err(Error::NeedInit(self.init_path_take()?));
        }

        let config = self.config(Some(InitOptions::from_file()))?;
        let mut old_flags = config.cflags.as_ref().unwrap().split_whitespace().collect::<HashSet<&str>>();

        if let Some(f) = cflags
            .split_whitespace()
            .map(|f| { if !f.starts_with("-") { format!("-{}", f)} else { f.to_string() }})
            .find(|f| !old_flags.remove(f.as_str())) 
        {
            return Err(CFlagNotFound(f.into())); 
        }

        let new_flags = old_flags.into_iter().collect::<Vec<&str>>().join(" ");

        println!("New cflags {}", new_flags);

        let build = ScriptBuilder::new(ScriptKind::Build)
            .cflags(&new_flags)
            .src_file_names(&config.src_file_names)
            .target_name(&config.target_executable_name)
            .build()
            .unwrap();

        fs::write(init_path.join("cflagswin"), new_flags)?;
        fs::write(self.root_folder_canonical.as_ref()?.join("build.bat"), build)?;

        Ok(())
    }
}