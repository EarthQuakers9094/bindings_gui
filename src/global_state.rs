use std::{
    borrow::Cow,
    collections::BTreeSet,
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
};

use anyhow::{Context, Result};
use egui::Ui;
use egui_toast::{Toast, Toasts};

use crate::{
    bindings::Binding, bindings::BindingsMap, bindings::SaveData, component::EventStream,
    ProgramError, Tab,
};

pub enum GlobalEvents {
    AddBinding(Binding, String),
    RemoveBinding(Binding, String),
    AddCommand(String),
    RemoveCommand(String),
    DisplayError(String),
}

#[derive(Debug, Default)]
pub struct State {
    pub save_file: PathBuf,
    pub url: Option<String>,
    pub commands: BTreeSet<String>,
    pub bindings: BindingsMap,
}

impl State {
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
        }
    }

    pub fn write_out(&self) -> Result<()> {
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

    fn to_bindings(&self) -> SaveData {
        SaveData {
            url: Cow::Borrowed(&self.url),
            commands: Cow::Borrowed(&self.commands),
            command_to_bindings: Cow::Borrowed(&self.bindings.command_to_bindings),
        }
    }

    fn from_bindings(bindings: SaveData, path: PathBuf) -> Self {
        Self {
            save_file: path,
            url: bindings.url.into_owned(),
            commands: bindings.commands.into_owned(),
            bindings: bindings.command_to_bindings.into_owned().into(),
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
}
