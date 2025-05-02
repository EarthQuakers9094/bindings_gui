use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
};

use anyhow::{Context, Result};
use egui::Ui;
use egui_toast::{Toast, Toasts};

use crate::{
    bindings::Binding, bindings::BindingsMap, component::EventStream, bindings::Bindings, ProgramError, Tab,
};

pub enum GlobalEvents {
    AddBinding(Binding, String),
    RemoveBinding(Binding, String),
    AddCommand(String),
    RemoveCommand(String),
    DisplayError(String),
}

#[derive(Debug)]
pub struct Views {
    pub save_file: PathBuf,
    pub url: Option<String>,
    pub commands: BTreeSet<String>,
    pub bindings: BindingsMap,
}

impl Views {
    pub fn display_tab(&mut self, ui: &mut Ui, tab: &mut Tab, toasts: &mut Toasts) -> Result<()> {
        let mut events = EventStream::new();

        tab.tab.render(ui, self, &mut events);

        let mut update = false;

        for e in events.drain() {
            update |= self.handle_event(e, toasts); // don't do any because any terminates early
        }

        if update {
            self.write_out()?
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
                self.commands.insert(command);
                true
            }
            GlobalEvents::RemoveCommand(command) => {
                self.commands.remove(&command);
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
        }
    }

    pub fn write_out(&self) -> Result<()> {
        // let mut dir = self.directory.clone();

        create_dir_all(self.save_file.parent().unwrap())?;

        let mut file =
            File::create(&self.save_file).with_context(|| "failed to create file to save to")?;

        file.write_all(
            serde_json::to_string(&self.to_bindings())
                .unwrap()
                .as_bytes(),
        )
        .with_context(|| "failed to save to disk")?;

        Ok(())
    }

    fn to_bindings(&self) -> Bindings {
        Bindings {
            url: Cow::Borrowed(&self.url),
            commands: Cow::Borrowed(&self.commands),
            command_to_bindings: Cow::Borrowed(&self.bindings.command_to_bindings),
        }
    }

    fn from_bindings(bindings: Bindings, path: PathBuf) -> Self {
        let mut binding_to_command = BTreeMap::new();

        for (command, bindings) in bindings.command_to_bindings.iter() {
            for b in bindings {
                binding_to_command
                    .entry((b.controller, b.button))
                    .or_insert(Vec::new())
                    .push((command.clone(), b.when));
            }
        }

        Self {
            save_file: path,
            url: bindings.url.into_owned(),
            commands: bindings.commands.into_owned(),
            bindings: BindingsMap {
                command_to_bindings: bindings.command_to_bindings.into_owned(),
                binding_to_command,
            },
            ..Default::default()
        }
    }

    pub fn from_directory(mut path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            return Err(ProgramError::NotDirectory(path))?;
            // return format!("{} is not a directory", path.display())?;
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

        let bindings: Bindings = serde_json::from_str(&file)?;

        Ok(Self::from_bindings(bindings, path))
    }
}

impl Default for Views {
    fn default() -> Self {
        Self {
            save_file: Default::default(),
            url: Default::default(),
            commands: Default::default(),
            bindings: Default::default(),
        }
    }
}
