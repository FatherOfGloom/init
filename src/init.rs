use std::{borrow::Cow, cell::{LazyCell, OnceCell, Ref, RefCell, RefMut}, collections::{HashMap, HashSet}, fmt::Write, fs, path::{Path, PathBuf}, process::Command};

use crate::{common::{Error, TemplateKind}, dependency::{Dependencies, Dependency}, init::Error::{CFlagNotFound, CFlagNotUnique, ParseError}, script_builder::{ScriptBuilder, ScriptKind::{self}}};

pub(crate) struct ConfigCache<'a> {
    template_kind: TemplateKind,
    src_file_names: Cow<'a, str>,
    target_executable_name: Cow<'a, str>,
    cflags: Option<Cow<'a, str>>,
    deps: Option<Dependencies>,
    linker_flags: OnceCell<Cow<'a, str>>,
    include_flags: OnceCell<Cow<'a, str>>,
    build_script_cache: OnceCell<String>,
    run_script_cache: OnceCell<String>,
    setup_script_cache: OnceCell<String>,
}

impl ConfigCache<'_> {
    fn template(&self) -> &'static str {
        match self.template_kind {
            TemplateKind::Main => TEMPLATE_C_DEFAULT,
            TemplateKind::Raylib => TEMPLATE_C_RAYLIB
        }
    }

    fn cflags(&self) -> &str {
        self.cflags
            .as_ref()
            .map(|c| c.as_ref())
            .unwrap_or_else(|| Self::cflags_default(self.template_kind))
    }

    fn cflags_default(kind: TemplateKind) -> &'static str {
        match kind {
            TemplateKind::Main => TEMPLATE_CFLAGS_DEFAULT,
            TemplateKind::Raylib => TEMPLATE_CFLAGS_RAYLIB,
        }
    }

    fn cache(&self) -> String {
        format!(
            "template {}\n",
            self.template_kind.as_str()
        )
    }

    fn deps(&self) -> Option<String> {
        self.deps
            .as_ref()
            .map(|deps| {
                deps
                    .iter()
                    .map(|Dependency { name, url, no_root }| format!("{} {} {}", name, if *no_root { "nr" } else { "r" }, url))
                    .collect::<Vec<String>>()
                    .join("\n")
            })
    }

    fn linker_flags(&self) -> Option<&str> {
        self.deps.as_ref().map(|deps| self.linker_flags.get_or_init(|| {
            deps
                .iter()
                .map(|d| format!("-Llib\\{}\\lib", d.name))
                .collect::<Vec<String>>()
                .join(" ")
                .into()
        }).as_ref())
    }

    fn include_flags(&self) -> Option<&str> {
        self.deps.as_ref().map(|deps| self.include_flags.get_or_init(|| {
            deps
                .iter()
                .map(|d| format!("-Ilib\\{}\\include", d.name))
                .collect::<Vec<String>>()
                .join(" ")
                .into()
        }).as_ref())
    }

    fn build(&self) -> &str {
       self.build_script_cache.get_or_init(|| {
            let mut builder = ScriptBuilder::new(ScriptKind::Build)
                .cflags(self.cflags())
                .src_file_names(&self.src_file_names)
                .target_name(&self.target_executable_name);

            if let (Some(include_flags), Some(linker_flags)) = (self.include_flags(), self.linker_flags()) {
                builder = builder.include_flags(include_flags).linker_flags(linker_flags);
            } 

            builder.build().unwrap()
        })
    }

    fn run(&self) -> &str {
       self.run_script_cache.get_or_init(|| {
            ScriptBuilder::new(ScriptKind::Run)
                .target_name(&self.target_executable_name)
                .build()
                .unwrap()
        })
    }

    fn setup(&self) -> Option<&String> {
        self.deps.as_ref().map(|d| self.setup_script_cache.get_or_init(|| {
            ScriptBuilder::new(ScriptKind::Setup)
                .dep_urls(&d)
                .build()
                .unwrap()
        }))
    }

    fn gitignore(&self) -> &str {
        TEMPLATE_GITIGNORE_DEFAULT
    }

    fn clang_format(&self) -> &str {
        TEMPLATE_CLANG_FORMAT_DEFAULT
    }
}

pub(crate) struct Init<'a> {
    root_folder_display: LazyCell<Result<PathBuf, std::io::Error>>,
    root_folder_canonical: LazyCell<Result<PathBuf, std::io::Error>>,
    config: RefCell<Option<ConfigCache<'a>>>,
    init_path_cache: RefCell<Option<PathBuf>>,
    updated_flags: Option<String>,
    need_update_deps: bool,
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
const TEMPLATE_C_RAYLIB: &'static str = include_str!("templates/main_raylib.c"); 
const TEMPLATE_CFLAGS_DEFAULT: &'static str = "-Wall -Wextra -Wpedantic -fanalyzer -std=c11";
const TEMPLATE_CFLAGS_RAYLIB: &'static str = "-Wall -Wextra -Wpedantic -fanalyzer -std=c11 -lraylib -lgdi32 -lwinmm";
const TEMPLATE_FILE_NAMES_DEFAULT: &'static str = "main.c";
const TEMPLATE_CLANG_FORMAT_DEFAULT: &'static str = include_str!("templates/clang_format_default");
const TEMPLATE_GITIGNORE_DEFAULT: &'static str = include_str!("templates/gitignore_default");

impl<'a> Init<'a> {
    pub(crate) fn new() -> Self {
        Init { 
            root_folder_display: LazyCell::new(|| { std::path::absolute(".") }),
            root_folder_canonical: LazyCell::new(|| { std::fs::canonicalize(".") }),
            config: RefCell::new(None),
            init_path_cache: RefCell::new(None),
            updated_flags: None,
            need_update_deps: false,
        }
    }

    fn init_path(&self) -> Result<Ref<'_, Path>, Error> {
        if self.init_path_cache.borrow().is_none() {
            *self.init_path_cache.borrow_mut() = Some(self.root_folder_canonical.as_ref()?.join(".init"));
        }

        Ok(Ref::map(self.init_path_cache.borrow(), |opt| opt.as_ref().map(|p| p.as_path()).unwrap()))
    }

    fn init_config_if_none(&self, init_options: Option<InitOptions>) -> Result<(), Error> {
        if self.config.borrow().is_none() {
            let root_path_canonical = self.root_folder_canonical.as_ref()?;
            let init_options = init_options.unwrap_or_default();

            let (cflags, exe, kind, deps) = match init_options {
                InitOptions::FromCommand { target_executable_name, template_kind } => { 
                    (
                        Cow::Borrowed(ConfigCache::cflags_default(template_kind)), 
                        target_executable_name.map(|n| Cow::Owned(n)),
                        template_kind, 
                        Dependencies::from_template(template_kind)
                    )
                },
                InitOptions::FromFile => {
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

                    let deps = Dependencies::from_file_if_exists(&init_path.join("deps"))?;

                    (Cow::Owned(cflags), None, kind, deps)
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
                deps: deps,
                build_script_cache: OnceCell::new(),
                run_script_cache: OnceCell::new(),
                setup_script_cache: OnceCell::new(),
                include_flags: OnceCell::new(),
                linker_flags: OnceCell::new()
            }); 
        }
        Ok(())
    }

    fn config(&self, init_options: Option<InitOptions>) -> Result<Ref<'_, ConfigCache<'a>>, Error> {
        self.init_config_if_none(init_options)?;

        Ok(Ref::map(self.config.borrow(), |c| c.as_ref().unwrap()))
    }

    fn config_mut(&self, init_options: Option<InitOptions>) -> Result<RefMut<'_, ConfigCache<'a>>, Error> {
        self.init_config_if_none(init_options)?;

        Ok(RefMut::map(self.config.borrow_mut(), |c| c.as_mut().unwrap()))
    }

    pub(crate) fn init(&mut self, options: Option<InitOptions>) -> Result<(), Error> {
        let root_folder_canonical = self.root_folder_canonical.as_ref()?;
        let root_folder_display = self.root_folder_display.as_ref()?;

        eprintln!("Initializing a C project in {}...", root_folder_display.display());

        let init_path = self.init_path()?;

        if init_path.exists() {
            return Err(Error::AlreadyExists(init_path.to_owned()));
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

            if let Some(deps) = config.deps() {
                fs::write(init_path.join("deps"), deps)?;
            }

            fs::write(root_folder_canonical.join("build.bat"), &*config.build())?;
            fs::write(root_folder_canonical.join("run.bat"), &*config.run())?;

            if let Some(setup) = config.setup() {
                fs::write(root_folder_canonical.join("setup.bat"), setup)?;
            }

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

    pub(crate) fn add_cflags(&mut self, cflags: &str) -> Result<(), Error> {
        self.updated_flags = Some({
            let init_path = self.init_path()?;

            if !init_path.exists() {
                return Err(Error::NeedInit(init_path.to_owned()));
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
            new_flags
        });
        
        Ok(())
    }

    pub(crate) fn remove_cflags(&mut self, cflags: &str) -> Result<(), Error> {
        self.updated_flags = Some({
            let init_path = self.init_path()?;

            if !init_path.exists() {
                return Err(Error::NeedInit(init_path.to_owned()));
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
            new_flags
        });

        Ok(())
    }

    pub(crate) fn reset_cflags(&mut self) -> Result<(), Error> {
        self.updated_flags = Some({
            let init_path = self.init_path()?;

            if !init_path.exists() {
                return Err(Error::NeedInit(init_path.to_owned()));
            }

            let config = self.config(Some(InitOptions::from_file()))?;

            ConfigCache::cflags_default(config.template_kind).to_owned()
        });

        Ok(())
    }

    pub(crate) fn list_cflags(&self) -> Result<(), Error> {
        let init_path = self.init_path()?;

        if !init_path.exists() {
            return Err(Error::NeedInit(init_path.to_owned()));
        }

        let config = self.config(Some(InitOptions::from_file()))?;

        if let Some(cflags) = config.cflags.as_ref() {
            println!("You currently have these cflags set: '{}'.", cflags);
        } else {
            println!("You don't have any cflags set");
        }

        Ok(())
    }

    pub(crate) fn add_dependencies(&mut self, dependencies: &str) -> Result<(), Error> {
        self.need_update_deps = {
            let init_path = self.init_path()?;

            if !init_path.exists() {
                return Err(Error::NeedInit(init_path.to_owned()));
            }

            let mut config = self.config_mut(Some(InitOptions::from_file()))?;

            if config.deps.is_none() {
                config.deps = Some(Dependencies::empty());
            }

            let deps = config.deps.as_mut().unwrap();

            let len = deps.len(); 

            for dep in dependencies.split_whitespace() {
                deps.add_dependency(None, dep, false)?;
            }

            len != deps.len()
        };

        Ok(())
    }

    pub(crate) fn remove_dependencies_by_names(&mut self, dep_names: &str) -> Result<(), Error> {
        self.need_update_deps = {
            let init_path = self.init_path()?;

            if !init_path.exists() {
                return Err(Error::NeedInit(init_path.to_owned()));
            }

            let mut config = self.config_mut(Some(InitOptions::from_file()))?;

            if config.deps.is_none() {
                config.deps = Some(Dependencies::empty());
            }


            let deps = config.deps.as_mut().unwrap();
            
            let mut deps_to_remove = Vec::with_capacity(deps.len());

            {
                let deps_map: HashMap<&str, usize> = deps
                    .iter()
                    .enumerate()
                    .map(|(idx, dep)| (dep.name.as_ref(), idx))
                    .collect();

                for dep_name in dep_names.split_whitespace() {
                    match deps_map.get(dep_name) {
                        Some(dep_to_remove) => deps_to_remove.push(*dep_to_remove),
                        None => return Err(Error::DepNotFoundByName(dep_name.into())),
                    }
                }
            }

            deps.remove_dependencies(&deps_to_remove);

            !deps_to_remove.is_empty()
        };

        Ok(())
    }

    pub(crate) fn reset_dependencies(&mut self) -> Result<(), Error> {
        self.need_update_deps = {
            let init_path = self.init_path()?;

            if !init_path.exists() {
                return Err(Error::NeedInit(init_path.to_owned()));
            }

            let mut config = self.config_mut(Some(InitOptions::from_file()))?;

            config.deps = Dependencies::from_template(config.template_kind);

            true
        };

        Ok(())
    }

    pub(crate) fn list_dependencies(&mut self) -> Result<(), Error> {
        let init_path = self.init_path()?;

        if !init_path.exists() {
            return Err(Error::NeedInit(init_path.to_owned()));
        }

        let config = self.config(Some(InitOptions::FromFile))?;

        if let Some(deps) = config
            .deps
            .as_ref()
            .map(|deps| { 
                let mut res = String::new();
                for dep in deps.iter() {
                    write!(&mut res, "{}: {}\n", dep.name, dep.url).unwrap();
                }
                res
            }) {
            println!("You currently have these dependencies set: '{}'.", deps);
            std::mem::forget(deps);
        } else {
            println!("You don't have any dependencies set.");
        }

        Ok(())
    }
}

impl Drop for Init<'_> {
    fn drop(&mut self) {
        if let Some(cflags) = &self.updated_flags {
            let config = self.config.borrow();
            let config = config.as_ref().unwrap();
            let init_path = self.init_path().unwrap();

            let mut build = ScriptBuilder::new(ScriptKind::Build)
                .cflags(&cflags)
                .src_file_names(&config.src_file_names)
                .target_name(&config.target_executable_name);

            if let (Some(include_flags), Some(linker_flags)) = (config.include_flags(), config.linker_flags()) {
                build = build.include_flags(include_flags).linker_flags(linker_flags);
            } 

            let build = build
                .build()
                .unwrap();

            fs::write(init_path.join("cflagswin"), cflags).unwrap();
            fs::write(self.root_folder_canonical.as_ref().unwrap().join("build.bat"), build).unwrap();
        }

        if self.need_update_deps && let Some(config) = self.config.borrow().as_ref() {
            let root_folder_canonical = self.root_folder_canonical.as_ref().unwrap();
            let init_path = self.init_path().unwrap();
            let deps_path = init_path.join("deps");

            if let Some(deps) = config.deps() {
                fs::write(deps_path, deps).unwrap();
            } else {
                if deps_path.try_exists().unwrap() {
                    fs::remove_file(deps_path).unwrap();
                }

                let setup_path = root_folder_canonical.join("setup.bat");

                if setup_path.try_exists().unwrap() {
                    fs::remove_file(setup_path).unwrap();
                }

                let lib_path = root_folder_canonical.join("lib");

                if lib_path.try_exists().unwrap() {
                    fs::remove_dir_all(lib_path).unwrap(); 
                }
            }

            fs::write(root_folder_canonical.join("build.bat"), &*config.build()).unwrap();

            if let Some(setup) = config.setup() {
                fs::write(root_folder_canonical.join("setup.bat"), setup).unwrap();
            }
        }
    }
}