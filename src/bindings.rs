use std::{borrow::Cow, collections::{BTreeMap, BTreeSet}, fmt::Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub enum Button {
    Button(u8),
    Pov(i16),
}

impl Display for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Button::Button(b) => b.fmt(f),
            Button::Pov(pov) => match pov {
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
    pub when: RunWhen,
}

impl Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.controller, self.button, self.when)
    }
}

#[derive(Debug)]
pub(crate) struct BindingsMap {
    pub command_to_bindings: BTreeMap<String, Vec<Binding>>,
    pub binding_to_command: BTreeMap<(u8, Button), Vec<(String, RunWhen)>>,
}

impl Default for BindingsMap {
    fn default() -> Self {
        Self {
            command_to_bindings: Default::default(),
            binding_to_command: Default::default(),
        }
    }
}

impl BindingsMap {
    pub(crate) fn add_binding(&mut self, command: String, binding: Binding) -> bool {
        if self
            .command_to_bindings
            .get(&command)
            .unwrap_or(&Vec::new())
            .contains(&binding)
        {
            false
        } else {
            self.command_to_bindings
                .entry(command.clone())
                .or_insert(Vec::new())
                .push(binding);

            self.binding_to_command
                .entry((binding.controller, binding.button))
                .or_insert(Vec::new())
                .push((command.clone(), binding.when));

            true
        }
    }

    pub(crate) fn remove_binding(&mut self, command: &String, binding: Binding) {
        self.command_to_bindings
            .get_mut(command)
            .unwrap()
            .retain(|b| *b != binding);
        self.binding_to_command
            .get_mut(&(binding.controller, binding.button))
            .unwrap()
            .retain(|(c, when): &(String, RunWhen)| !(command == c && *when == binding.when));
    }

    pub(crate) fn is_used(&self, command: &String) -> bool {
        self.command_to_bindings
            .get(command)
            .map_or(false, |l| !l.is_empty())
    }

    pub(crate)  fn has_binding(&self, command: &String, binding: Binding) -> bool {
        self.command_to_bindings
            .get(command)
            .map_or(false, |bindings| bindings.contains(&binding))
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Bindings<'a> {
    pub(crate) url: Cow<'a, Option<String>>,
    pub(crate) commands: Cow<'a, BTreeSet<String>>,
    pub(crate) command_to_bindings: Cow<'a, BTreeMap<String, Vec<Binding>>>,
}