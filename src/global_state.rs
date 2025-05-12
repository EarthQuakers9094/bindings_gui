use std::{
    borrow::Cow,
    collections::BTreeSet,
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
    process::{Child, Command},
    rc::Rc,
};

use anyhow::{Context, Result};
use bumpalo::Bump;
use egui::Ui;
use egui_toast::{Toast, Toasts};

use crate::{
    bindings::{self, Binding, BindingsMap, ControllerType, Profile, SaveData},
    component::EventStream,
    ProgramError, Tab,
};

#[derive(Debug, Clone)]
pub enum GlobalEvents {
    AddBinding(Binding, Rc<String>),
    RemoveBinding(Binding, Rc<String>),
    AddCommand(String),
    RemoveCommand(Rc<String>),
    DisplayError(String),
    Save,
    RenameCommand(Rc<String>, Rc<String>),
    AddProfile(String),
    SetProfile(Rc<String>),
}

#[derive(Debug)]
pub struct State {
    pub deploy_dir: PathBuf,
    pub url: Option<String>,
    pub syncing: bool,
    pub commands: BTreeSet<Rc<String>>,
    pub bindings: BindingsMap,
    pub controllers: [ControllerType; 5],
    pub controller_names: [Rc<String>; 5],
    pub sync_process: Option<Child>,
    pub profile: Rc<String>,
    pub profiles: Vec<Rc<String>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            url: Default::default(),
            syncing: true,
            commands: Default::default(),
            bindings: Default::default(),
            controllers: Default::default(),
            controller_names: Default::default(),
            sync_process: Default::default(),
            deploy_dir: PathBuf::default(),
            profile: Rc::new("default".to_string()),
            profiles: Default::default(),
        }
    }
}

impl State {
    pub fn display_tab(
        &mut self,
        ui: &mut Ui,
        tab: &mut Tab,
        toasts: &mut Toasts,
        arena: &Bump,
    ) -> Result<()> {
        let mut events = EventStream::new();

        tab.tab.render(ui, self, &events, arena);

        let mut update = false;

        for e in events.drain() {
            update |= self.handle_event(e, toasts); // don't do any because any terminates early
        }

        if update {
            self.write_out(arena)?
        }

        Ok(())
    }

    pub fn handle_event(&mut self, event: GlobalEvents, toasts: &mut Toasts) -> bool {
        match event {
            GlobalEvents::AddBinding(binding, command) => {
                self.bindings.add_binding(command, binding);
                true
            }
            GlobalEvents::RemoveBinding(binding, command) => {
                self.bindings.remove_binding(&command, binding);
                true
            }
            GlobalEvents::AddCommand(command) => {
                self.commands.insert(Rc::new(command));
                true
            }
            GlobalEvents::RemoveCommand(command) => {
                self.commands.remove(&command);
                self.bindings.remove_command(&command);
                true
            }
            GlobalEvents::DisplayError(error) => {
                toasts.add(Toast {
                    kind: egui_toast::ToastKind::Error,
                    text: error.into(),
                    ..Default::default()
                });
                false
            }
            GlobalEvents::Save => true,
            GlobalEvents::RenameCommand(old, new) => {
                self.bindings.rename_binding(old.clone(), new.clone());
                self.commands.remove(&old);
                self.commands.insert(new);
                true
            }
            GlobalEvents::AddProfile(profile) => {
                self.profiles.push(Rc::new(profile));
                false
            }
            GlobalEvents::SetProfile(profile) => {
                match self.change_profile(profile) {
                    Ok(()) => {}
                    Err(err) => {
                        self.handle_event(GlobalEvents::DisplayError(err.to_string()), toasts);
                    }
                };
                false
            }
        }
    }

    pub fn change_profile(&mut self, profile: Rc<String>) -> Result<()> {
        self.profile = profile.clone();

        let mut path = self.deploy_dir.to_path_buf();

        path.push("profile");

        let mut file: File = File::create(path).with_context(|| "failed to create profile file")?;

        file.write_all(profile.as_bytes())?;

        println!("just wrote file");

        let profile: Profile<'static> = self.get_profile(profile.as_str())?;

        self.bindings = profile.command_to_bindings.into_owned().into();
        self.controller_names = profile.controller_names.into_owned();
        self.controllers = profile.controllers.into_owned();

        Ok(())
    }

    pub fn get_profile(&self, profile: &str) -> Result<Profile<'static>> {
        Profile::get_from(&self.deploy_dir, profile)
    }

    pub fn write_out(&mut self, arena: &Bump) -> Result<()> {
        let mut save_file = self.deploy_dir.clone(); // fix the clones in this function

        save_file.push("bindings.json");

        let mut profile = self.deploy_dir.clone();

        profile.push("bindings");
        profile.push(bumpalo::format!(in &arena, "{}.json", self.profile).as_str());

        create_dir_all(save_file.parent().unwrap())?;

        let mut file =
            File::create(&save_file).with_context(|| "failed to create file to save to")?;

        file.write_all(
            serde_json::to_string(&self.to_savedata())
                .unwrap()
                .as_bytes(),
        )?;

        create_dir_all(profile.parent().unwrap())?;

        let mut file =
            File::create(&profile).with_context(|| "failed to create file to savce to")?;

        file.write_all(
            serde_json::to_string(&self.to_profile_data())
                .unwrap()
                .as_bytes(),
        )
        .with_context(|| "failed to save to disk")?;

        match &self.url {
            Some(url) if self.syncing => {
                if let Some(child) = &mut self.sync_process {
                    child.kill()?
                }

                self.sync_process = Some(
                    Command::new("scp")
                        .arg(save_file.as_os_str())
                        .arg(profile.as_os_str())
                        .arg(
                            bumpalo::format!(in &arena, "admin@{}:/home/lvuser/deploy/bindings.json", url)
                                .as_str(),
                        )
                        .spawn()?,
                );
            }
            _ => {}
        }

        Ok(())
    }

    fn to_profile_data(&self) -> Profile {
        Profile {
            command_to_bindings: Cow::Borrowed(&self.bindings.command_to_bindings),
            controllers: Cow::Borrowed(&self.controllers),
            controller_names: Cow::Borrowed(&self.controller_names),
        }
    }

    fn to_savedata(&self) -> SaveData {
        SaveData {
            url: Cow::Borrowed(&self.url),
            commands: Cow::Borrowed(&self.commands),
        }
    }

    fn from_bindings(
        bindings: SaveData,
        profile: Profile,
        profiles: Vec<Rc<String>>,
        profile_name: String,
        path: PathBuf,
    ) -> Self {
        Self {
            url: bindings.url.into_owned(),
            commands: bindings.commands.into_owned(),
            bindings: profile.command_to_bindings.into_owned().into(),
            controllers: profile.controllers.into_owned(),
            controller_names: profile.controller_names.into_owned(),
            syncing: true,
            sync_process: Default::default(),
            deploy_dir: path,
            profile: Rc::new(profile_name),
            profiles,
        }
    }

    pub fn from_directory(mut path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            return Err(ProgramError::NotDirectory(path))?;
        }

        path.push("src");
        path.push("main");
        path.push("deploy");

        path.push("profile");

        if path.is_dir() {
            // return is here just to convice the borrow checker that this path never
            // continues executing the function
            return Err(ProgramError::ExistingDirectoryAt(path))?;
        }

        let profile_name = match read_to_string(&path) {
            Ok(a) => a,
            Err(_err) => {
                let mut file = File::create_new(&path)?;

                file.write_all("default".as_bytes())?;

                "default".to_string()
            }
        };

        path.pop();

        let bindings = match SaveData::from_directory(&path)? {
            Some(a) => a,
            None => {
                return Ok(Self {
                    deploy_dir: path,
                    profile: Rc::new(profile_name),
                    ..Default::default()
                })
            }
        };

        let mut profiles = Profile::get_profiles(&path)?;

        if !profiles
            .iter()
            .map(|s| s.as_str())
            .any(|s| s == profile_name.as_str())
        {
            profiles.push(Rc::new(profile_name.clone()));
        }

        let profile = Profile::get_from(&path, &profile_name)?;

        Ok(Self::from_bindings(
            bindings,
            profile,
            profiles,
            profile_name,
            path,
        ))
    }

    pub fn valid_binding(&self, controller: u8, binding: bindings::Button) -> bool {
        self.controllers
            .get(controller as usize)
            .map(|c| c.valid_binding(binding))
            .unwrap_or(false)
    }

    pub fn controller_name(&self, controller: u8) -> Rc<String> {
        let name = &self.controller_names[controller as usize];

        if name.is_empty() {
            return Rc::new(controller.to_string());
        }

        name.clone()
    }
}
