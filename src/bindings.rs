use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    rc::Rc,
};

use bumpalo::Bump;
use egui::{ComboBox, Id, Ui};
use serde::{Deserialize, Serialize};

use crate::{
    global_state::State,
    search_selector::{self, SingleCache},
};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub enum ButtonLocation {
    Button,
    Analog,
    Pov,
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub struct Button {
    pub(crate) button: i16,
    pub(crate) location: ButtonLocation,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            button: 1,
            location: ButtonLocation::Button,
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
                .selected_text(self.get_str())
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
    pub during: RunWhen, // bad name because "when" is a reserved keyword in kotlin and im lazy
}

impl Binding {
    pub fn show<'a>(&self, env: &State, arena: &'a Bump) -> &'a str {
        bumpalo::format!(in arena,
            "on {} to {} {}",
            env.controller_name(self.controller),
            env.controllers[self.controller as usize].button_name(&self.button, arena),
            self.during
        )
        .into_bump_str()
    }
}

#[derive(Debug, Default)]
pub(crate) struct BindingsMap {
    pub command_to_bindings: BTreeMap<Rc<String>, Vec<Binding>>,
    pub binding_to_commands: BTreeMap<(u8, Button), Vec<(Rc<String>, RunWhen)>>,
}

impl From<BTreeMap<Rc<String>, Vec<Binding>>> for BindingsMap {
    fn from(command_to_bindings: BTreeMap<Rc<String>, Vec<Binding>>) -> Self {
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
    pub(crate) fn add_binding(&mut self, command: Rc<String>, binding: Binding) {
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
                .push((command, binding.during));
        }
    }

    pub(crate) fn remove_command(&mut self, command: &String) {
        self.command_to_bindings.remove(command);
        for commands in self.binding_to_commands.values_mut() {
            commands.retain(|(c, _)| c.as_ref() != command);
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

    pub(crate) fn rename_binding(&mut self, from: Rc<String>, to: Rc<String>) {
        let bindings = self.command_to_bindings.remove(&from).unwrap();

        for binding in &bindings {
            for (command, when) in self
                .binding_to_commands
                .get_mut(&(binding.controller, binding.button)).unwrap()
            {
                if *command == from {
                    *command = to.clone();
                }
            }
        }

        self.command_to_bindings.insert(to, bindings);
    }

    pub(crate) fn remove_binding(&mut self, command: &String, binding: Binding) {
        self.command_to_bindings
            .get_mut(command)
            .unwrap()
            .retain(|b| *b != binding);
        self.binding_to_commands
            .get_mut(&(binding.controller, binding.button))
            .unwrap()
            .retain(|(c, when): &(Rc<String>, RunWhen)| {
                !(command == c.as_ref() && *when == binding.during)
            });
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum ControllerType {
    Generic {
        buttons: u8,
    },
    XBox {
        sensitivity: f32,
    },
    #[default]
    NotBound,
}

impl ControllerType {
    fn num_buttons(&self) -> u8 {
        match self {
            ControllerType::Generic { buttons } => *buttons,
            ControllerType::XBox { .. } => 10,
            ControllerType::NotBound => 0,
        }
    }

    pub fn bound(&self) -> bool {
        !matches!(self, ControllerType::NotBound)
    }

    pub fn button_name<'a>(&self, button: &Button, arena: &'a Bump) -> &'a str {
        match button.location {
            ButtonLocation::Button => match self {
                ControllerType::Generic { .. } => {
                    bumpalo::format!(in arena, "{}", button.button).into_bump_str()
                }
                ControllerType::XBox { .. } => [
                    "a",
                    "b",
                    "x",
                    "y",
                    "left bumper",
                    "right bumper",
                    "back",
                    "start",
                    "left stick",
                    "right stick",
                ][button.button as usize - 1],
                ControllerType::NotBound => "ERROR",
            },
            ButtonLocation::Pov => match button.button {
                0 => "pov up",
                45 => "pov up right",
                90 => "pov right",
                135 => "pov down right",
                180 => "pov down",
                225 => "pov down left",
                270 => "pov left",
                315 => "pov up left",
                -1 => "no pov",
                _ => "ERROR",
            },
            ButtonLocation::Analog => match self {
                ControllerType::Generic { buttons: _ } => todo!(),
                ControllerType::XBox { .. } => match button.button {
                    2 => "left trigger",
                    3 => "right trigger",
                    _ => "invalid trigger",
                },
                ControllerType::NotBound => "ERROR",
            },
        }
    }

    pub fn enumerate_analog<'a>(&self, arena: &'a Bump) -> &'a mut dyn Iterator<Item = Button> {
        match self {
            ControllerType::Generic { buttons: _ } => arena.alloc([].into_iter()),
            ControllerType::XBox { .. } => arena.alloc(
                [
                    Button {
                        button: 2,
                        location: ButtonLocation::Analog,
                    },
                    Button {
                        button: 3,
                        location: ButtonLocation::Analog,
                    },
                ]
                .into_iter(),
            ),
            ControllerType::NotBound => arena.alloc([].into_iter()),
        }
    }

    pub fn enumerate_povs<'a>(&self, arena: &'a Bump) -> &'a mut dyn Iterator<Item = Button> {
        match self {
            Self::Generic { .. } | Self::XBox { .. } => arena.alloc(
                [-1, 0, 45, 90, 135, 180, 225, 270, 315]
                    .into_iter()
                    .map(|dir| Button {
                        button: dir,
                        location: ButtonLocation::Pov,
                    }),
            ),
            _ => arena.alloc(
                [-1, 0, 45, 90, 135, 180, 225, 270, 315]
                    .into_iter()
                    .map(|dir| Button {
                        button: dir,
                        location: ButtonLocation::Pov,
                    }),
            ),
        }
    }

    pub fn enumerate_buttons<'a>(&self, arena: &'a Bump) -> impl Iterator<Item = Button> + 'a {
        (1..=self.num_buttons())
            .map(|button| Button {
                button: button.into(),
                location: ButtonLocation::Button,
            })
            .chain(self.enumerate_povs(arena))
            .chain(self.enumerate_analog(arena))
    }

    // todo change u8 to actual button type to include pov
    pub fn show_button_selector(
        &self,
        id: Id,
        filter: &mut String,
        filter_cache: &mut SingleCache<String, Vec<(Rc<String>, Button)>>,
        button: &mut Button,
        ui: &mut Ui,
        arena: &Bump,
    ) {
        search_selector::search_selector(
            id,
            filter,
            button,
            self.enumerate_buttons(arena).map(|button| {
                (
                    Rc::new(self.button_name(&button, arena).to_string()),
                    button,
                )
            }),
            filter_cache,
            100.0,
            ui,
        );
    }

    pub fn valid_binding(&self, binding: Button) -> bool {
        match binding.location {
            ButtonLocation::Button => {
                1 <= binding.button && binding.button <= self.num_buttons().into()
            }
            ButtonLocation::Pov => match self {
                ControllerType::NotBound => false,
                _ => [-1, 0, 45, 90, 135, 180, 225, 270].contains(&binding.button),
            },
            ButtonLocation::Analog => match self {
                ControllerType::Generic { buttons: _ } => false,
                ControllerType::XBox { .. } => binding.button == 2 || binding.button == 3,
                ControllerType::NotBound => false,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct SaveData<'a> {
    pub(crate) url: Cow<'a, Option<String>>,
    pub(crate) commands: Cow<'a, BTreeSet<Rc<String>>>,
    pub(crate) command_to_bindings: Cow<'a, BTreeMap<Rc<String>, Vec<Binding>>>,
    pub(crate) controllers: Cow<'a, [ControllerType; 5]>,
    pub(crate) controller_names: Cow<'a, [Rc<String>; 5]>,
}
