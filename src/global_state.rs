use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
    process::{Child, Command},
    rc::Rc,
};

#[cfg(target_os = "windows")]
use std::os::windows::proccess::CommandExt;

use anyhow::{Context, Result};
use bumpalo::Bump;
use egui::Ui;
use egui_toast::{Toast, Toasts};

use crate::{
    bindings::{self, Binding, BindingsMap, ControllerType, Profile, SaveData},
    component::EventStream,
    constants::{Constants, OptionLocation},
    Component, ProgramError,
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
    AddOption(OptionLocation, Constants),
    AddOptionDriver(OptionLocation, Constants),
    RemoveOption(OptionLocation),
    RemoveOptionDriver(OptionLocation),
    SetStream(Rc<String>, u8, u8),
    AddStream(String),
    RenameStream(Rc<String>, Rc<String>),
    RemoveStream(Rc<String>),
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
    pub constants: Constants,
    pub driver_constants: Constants,
    pub stream_to_axis: BTreeMap<Rc<String>, (u8, u8)>,
    pub streams: BTreeSet<Rc<String>>,
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
            constants: Default::default(),
            driver_constants: Default::default(),
            stream_to_axis: Default::default(),
            streams: Default::default(),
        }
    }
}

impl State {
    pub fn display_tab(
        &mut self,
        ui: &mut Ui,
        tab: &mut Box<dyn Component<OutputEvents = GlobalEvents, Environment = Self>>,
        toasts: &mut Toasts,
        arena: &Bump,
    ) -> Result<()> {
        let mut events = EventStream::new();

        tab.render(ui, self, &events, arena);

        let mut update = false;

        for e in events.drain() {
            update |= self.handle_event(e, arena, toasts); // don't do any because any terminates early
        }

        if update {
            self.write_out(arena)?
        }

        Ok(())
    }

    pub fn handle_event(&mut self, event: GlobalEvents, arena: &Bump, toasts: &mut Toasts) -> bool {
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
                if let Err(err) = self.map_profiles(
                    |profile| {
                        let bindings = profile.command_to_bindings.to_mut().remove(&old);

                        if let Some(bindings) = bindings {
                            profile
                                .command_to_bindings
                                .to_mut()
                                .insert(new.clone(), bindings);
                        }
                    },
                    arena,
                ) {
                    self.handle_event(GlobalEvents::DisplayError(err.to_string()), arena, toasts);
                }

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
                        self.handle_event(
                            GlobalEvents::DisplayError(err.to_string()),
                            arena,
                            toasts,
                        );
                    }
                };
                false
            }
            GlobalEvents::AddOption(key, constant) => {
                if self.constants.add_option(key, constant) {
                    self.handle_event(
                        GlobalEvents::DisplayError("failed to add constants".to_string()),
                        arena,
                        toasts,
                    );
                    false
                } else {
                    true
                }
            }
            GlobalEvents::RemoveOption(key) => {
                self.constants.remove_key(&key);

                if let Err(err) = self.map_profiles(
                    |profile| {
                        profile.constants.to_mut().remove_key(&key);
                    },
                    arena,
                ) {
                    self.handle_event(GlobalEvents::DisplayError(err.to_string()), arena, toasts);
                }

                true
            }
            GlobalEvents::RemoveOptionDriver(key) => {
                self.driver_constants.remove_key(&key);

                true
            }
            GlobalEvents::AddOptionDriver(key, constant) => {
                if self.driver_constants.add_option(key, constant) {
                    self.handle_event(
                        GlobalEvents::DisplayError("failed to add constant".to_string()),
                        arena,
                        toasts,
                    );
                    false
                } else {
                    true
                }
            }
            GlobalEvents::SetStream(stream, controller, axis) => {
                self.stream_to_axis.insert(stream, (controller, axis));
                true
            }
            GlobalEvents::AddStream(stream) => {
                self.streams.insert(Rc::new(stream));
                true
            }
            GlobalEvents::RenameStream(from, to) => {
                self.streams.remove(&from);
                self.streams.insert(to.clone());

                let binding: Option<(u8, u8)> = self.stream_to_axis.remove(&from);

                if let Some(binding) = binding {
                    self.stream_to_axis.insert(to, binding);
                }

                true
            }
            GlobalEvents::RemoveStream(stream) => {
                self.streams.remove(&stream);
                true
            }
        }
    }

    pub fn change_profile(&mut self, profile: Rc<String>) -> Result<()> {
        self.profile = profile.clone();

        let mut path = self.deploy_dir.to_path_buf();

        path.push("profile");

        let mut file: File = File::create(path).with_context(|| "failed to create profile file")?;

        file.write_all(profile.as_bytes())?;

        let profile: Profile<'static> = self.get_profile(profile.as_str())?;

        self.set_fields_from_profile(profile);

        Ok(())
    }

    pub fn set_fields_from_profile(&mut self, profile: Profile<'_>) {
        self.bindings = profile.command_to_bindings.into_owned().into();
        self.controller_names = profile.controller_names.into_owned();
        self.controllers = profile.controllers.into_owned();
        self.constants = profile.constants.into_owned();
        self.stream_to_axis = profile.stream_to_axis.into_owned();
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
            serde_json::to_string_pretty(&self.to_savedata())
                .unwrap()
                .as_bytes(),
        )?;

        create_dir_all(profile.parent().unwrap())?;

        let mut file =
            File::create(&profile).with_context(|| "failed to create file to savce to")?;

        file.write_all(
            serde_json::to_string_pretty(&self.to_profile_data())
                .unwrap()
                .as_bytes(),
        )
        .with_context(|| "failed to save to disk")?;

        match &self.url {
            Some(url) if self.syncing => {
                if let Some(child) = &mut self.sync_process {
                    child.kill()?
                }

                let mut c = Command::new("scp");

                let command = c.arg("-r")
                        .arg(save_file.as_os_str())
                        .arg(bumpalo::format!(in &arena, "{}/bindings", self.deploy_dir.as_os_str().to_str().unwrap()).into_bump_str())
                        .arg(bumpalo::format!(in &arena, "admin@{}:/home/lvuser/deploy/", url).into_bump_str());

                #[cfg(target_os = "windows")]
                let command = command.creation_flags(0x08000000);

                self.sync_process = Some(command.spawn()?);
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
            constants: Cow::Borrowed(&self.driver_constants),
            stream_to_axis: Cow::Borrowed(&self.stream_to_axis),
        }
    }

    fn to_savedata(&self) -> SaveData {
        SaveData {
            url: Cow::Borrowed(&self.url),
            commands: Cow::Borrowed(&self.commands),
            constants: Cow::Borrowed(&self.constants),
            streams: Cow::Borrowed(&self.streams),
        }
    }

    pub fn map_profiles<F>(&mut self, mut f: F, arena: &Bump) -> Result<()>
    where
        F: FnMut(&mut Profile),
    {
        for ele in self
            .profiles
            .iter()
            .filter(|ele| ele.as_str() != self.profile.as_str())
        {
            let mut profile = self
                .get_profile(ele.as_str())
                .with_context(|| "failed to get profile")?;

            f(&mut profile);

            let mut path = self.deploy_dir.clone();

            path.push("bindings");

            path.push(bumpalo::format!(in arena, "{}.json", ele.as_str()).as_str());

            let mut file =
                File::create(path).with_context(|| "failed to create file to savce to")?;

            file.write_all(serde_json::to_string_pretty(&profile).unwrap().as_bytes())
                .with_context(|| "failed to save to disk")?;
        }

        let mut p = self.to_profile_data();

        f(&mut p);

        let p = p.get_owned();

        self.set_fields_from_profile(p);

        self.write_out(arena)?;

        Ok(())
    }

    pub fn enumerate_profiles(&self) -> impl Iterator<Item = Result<Profile>> {
        self.profiles
            .iter()
            .filter(|ele| ele.as_str() != self.profile.as_str())
            .map(|profile| self.get_profile(profile.as_str()))
            .chain([Ok(self.to_profile_data())])
    }

    pub fn is_used(&self, command: &Rc<String>) -> Result<bool> {
        for profile in self.enumerate_profiles() {
            let profile = profile?;

            if profile.command_to_bindings.contains_key(command) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn is_stream_used(&self, stream: &Rc<String>) -> Result<bool> {
        for profile in self.enumerate_profiles() {
            let profile = profile?;

            if profile.stream_to_axis.contains_key(stream) {
                return Ok(true);
            }
        }

        Ok(false)
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
            constants: bindings.constants.into_owned(),
            driver_constants: profile.constants.into_owned(),
            stream_to_axis: profile.stream_to_axis.into_owned(),
            streams: bindings.streams.into_owned(),
        }
    }

    pub fn from_directory(mut path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            return Err(ProgramError::NotDirectory(path))?;
        }

        path.push("src");
        path.push("main");
        path.push("deploy");

        create_dir_all(&path)?;

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
                let profile_name = Rc::new(profile_name);
                return Ok(Self {
                    deploy_dir: path,
                    profile: profile_name.clone(),
                    profiles: vec![profile_name],
                    ..Default::default()
                });
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
