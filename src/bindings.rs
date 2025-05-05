use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use egui::{ComboBox, Ui};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub enum ButtonLocation {
    Button,
    Pov,
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub struct Button {
    pub(crate) button: i16,
    pub(crate) location: ButtonLocation
}

impl Display for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.location {
            ButtonLocation::Button => self.button.fmt(f),
            ButtonLocation::Pov => match self.button {
                0 => write!(f, "up"),
                45 => write!(f, "up left"),
                90 => write!(f, "right"),
                135 => write!(f, "down right"),
                180 => write!(f, "down"),
                225 => write!(f, "down left"),
                270 => write!(f, "left"),
                315 => write!(f, "up left"),
                -1 => write!(f, "no pov"),
                _ => write!(f, "ERROR"),
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub enum RunWhen {
    OnTrue,
    OnFalse,
    WhileTrue,
    WhileFalse,
}

impl RunWhen {
    pub fn get_str(self) -> &'static str {
        match self {
            RunWhen::OnTrue => "on true",
            RunWhen::OnFalse => "on false",
            RunWhen::WhileTrue => "while true",
            RunWhen::WhileFalse => "while false",
        }
    }

    pub fn enumerate() -> impl Iterator<Item = RunWhen> {
        [
            RunWhen::OnTrue,
            RunWhen::OnFalse,
            RunWhen::WhileTrue,
            RunWhen::WhileFalse,
        ]
        .into_iter()
    }

    pub fn selection_ui(&mut self, ui: &mut Ui, id: impl std::hash::Hash) {
        ui.push_id(id, |ui| {
            ComboBox::from_label("")
                .selected_text(format!("{}", self))
                .show_ui(ui, |ui| {
                    for i in RunWhen::enumerate() {
                        ui.selectable_value(self, i, i.get_str());
                    }
                });
        });
    }
}

impl Display for RunWhen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_str())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub struct Binding {
    pub controller: u8,
    pub button: Button,
    pub during: RunWhen, // bad name because when is a reserved keyword in kotlin and im lazy
}

impl Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.controller, self.button, self.during)
    }
}

#[derive(Debug, Default)]
pub(crate) struct BindingsMap {
    pub command_to_bindings: BTreeMap<String, Vec<Binding>>,
    pub binding_to_commands: BTreeMap<(u8, Button), Vec<(String, RunWhen)>>,
}

impl From<BTreeMap<String, Vec<Binding>>> for BindingsMap {
    fn from(command_to_bindings: BTreeMap<String, Vec<Binding>>) -> Self {
        let mut binding_to_command = BTreeMap::new();

        for (command, bindings) in &command_to_bindings {
            for b in bindings {
                binding_to_command
                    .entry((b.controller, b.button))
                    .or_insert(Vec::new())
                    .push((command.clone(), b.during));
            }
        }

        BindingsMap {
            command_to_bindings,
            binding_to_commands: binding_to_command,
        }
    }
}

impl BindingsMap {
    pub(crate) fn add_binding(&mut self, command: String, binding: Binding) {
        if !self
            .command_to_bindings
            .get(&command)
            .unwrap_or(&Vec::new())
            .contains(&binding)
        {
            self.command_to_bindings
                .entry(command.clone())
                .or_default()
                .push(binding);

            self.binding_to_commands
                .entry((binding.controller, binding.button))
                .or_default()
                .push((command.clone(), binding.during));
        }
    }

    pub(crate) fn remove_command(&mut self, command: &String) {
        self.command_to_bindings.remove(command);
        for (_, commands) in &mut self.binding_to_commands {
            commands.retain(|(c, _)| c != command);
        }
    }


    pub(crate) fn bindings_for_command(
        &self,
        command: &String,
    ) -> impl Iterator<Item = Binding> + '_ {
        self.command_to_bindings
            .get(command)
            .into_iter()
            .flatten()
            .cloned()
    }

    pub(crate) fn remove_binding(&mut self, command: &String, binding: Binding) {
        self.command_to_bindings
            .get_mut(command)
            .unwrap()
            .retain(|b| *b != binding);
        self.binding_to_commands
            .get_mut(&(binding.controller, binding.button))
            .unwrap()
            .retain(|(c, when): &(String, RunWhen)| !(command == c && *when == binding.during));
    }

    pub(crate) fn is_used(&self, command: &String) -> bool {
        self.command_to_bindings
            .get(command)
            .is_some_and(|l| !l.is_empty())
    }

    pub(crate) fn has_button(&self, button: (u8, Button)) -> bool {
        self.binding_to_commands.contains_key(&button)
    }

    pub(crate) fn has_binding(&self, command: &String, binding: Binding) -> bool {
        self.command_to_bindings
            .get(command)
            .is_some_and(|bindings| bindings.contains(&binding))
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct SaveData<'a> {
    pub(crate) url: Cow<'a, Option<String>>,
    pub(crate) commands: Cow<'a, BTreeSet<String>>,
    pub(crate) command_to_bindings: Cow<'a, BTreeMap<String, Vec<Binding>>>,
}
