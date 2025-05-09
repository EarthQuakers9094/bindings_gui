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
    bindings::{self, Binding, BindingsMap, ControllerType, SaveData},
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
}

#[derive(Debug)]
pub struct State {
    pub save_file: PathBuf,
    pub url: Option<String>,
    pub syncing: bool,
    pub commands: BTreeSet<Rc<String>>,
    pub bindings: BindingsMap,
    pub controllers: [ControllerType; 5],
    pub controller_names: [Rc<String>; 5],
    pub sync_process: Option<Child>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            save_file: Default::default(),
            url: Default::default(),
            syncing: true,
            commands: Default::default(),
            bindings: Default::default(),
            controllers: Default::default(),
            controller_names: Default::default(),
            sync_process: Default::default(),
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
        }
    }

    pub fn write_out(&mut self, arena: &Bump) -> Result<()> {
        create_dir_all(self.save_file.parent().unwrap())?;

        let mut file =
            File::create(&self.save_file).with_context(|| "failed to create file to save to")?;

        file.write_all(
            serde_json::to_string(&self.to_bindings())
                .unwrap()
                .as_bytes(),
        )
        .with_context(|| "failed to save to disk")?;

        match &self.url {
            Some(url) if self.syncing => {
                match &mut self.sync_process {
                    Some(child) => {child.kill()?},
                    None => {},
                }

                self.sync_process = Some(
                    Command::new("scp")
                        .arg(self.save_file.as_os_str())
                        .arg(
                            bumpalo::format!(in &arena, "lvuser@{}:~/deploy/bindings.json", url)
                                .as_str(),
                        )
                        .spawn()?,
                );
            }
            _ => {}
        }

        Ok(())
    }

    fn to_bindings(&self) -> SaveData {
        SaveData {
            url: Cow::Borrowed(&self.url),
            commands: Cow::Borrowed(&self.commands),
            command_to_bindings: Cow::Borrowed(&self.bindings.command_to_bindings),
            controllers: Cow::Borrowed(&self.controllers),
            controller_names: Cow::Borrowed(&self.controller_names),
        }
    }

    fn from_bindings(bindings: SaveData, path: PathBuf) -> Self {
        Self {
            save_file: path,
            url: bindings.url.into_owned(),
            commands: bindings.commands.into_owned(),
            bindings: bindings.command_to_bindings.into_owned().into(),
            controllers: bindings.controllers.into_owned(),
            controller_names: bindings.controller_names.into_owned(),
            syncing: true,
            sync_process: Default::default(),
        }
    }

    pub fn from_directory(mut path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            return Err(ProgramError::NotDirectory(path))?;
        }

        path.push("src");
        path.push("main");
        path.push("deploy");
        path.push("bindings.json");

        if path.is_dir() {
            // return is here just to convice the borrow checker that this path never
            // continues executing the function
            return Err(ProgramError::ExistingDirectoryAt(path))?;
        }

        if !path.exists() {
            return Ok(Self {
                save_file: path,
                ..Default::default()
            });
        }

        let file = read_to_string(&path)?;

        let bindings: SaveData = serde_json::from_str(&file)?;

        Ok(Self::from_bindings(bindings, path))
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
