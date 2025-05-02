use std::{
    arch::x86_64,
    collections::{BTreeMap, BTreeSet},
};

use egui::{
    ahash::{HashMap, HashMapExt},
    cache::FrameCache,
    popup_below_widget, ComboBox, DragValue, Label, ScrollArea, Sense, Slider, Ui,
};

use crate::{Binding, Button, RunWhen, Views};

#[derive(Debug)]
pub struct EditingStates {
    command: String,
    when: RunWhen,
}

impl Default for EditingStates {
    fn default() -> Self {
        Self {
            command: "".to_string(),
            when: RunWhen::WhileTrue,
        }
    }
}

#[derive(Debug)]
struct SingleCash {
    last_key: Option<String>,
    value: Vec<String>,
    read: bool,
}

impl Default for SingleCash {
    fn default() -> Self {
        Self {
            last_key: Default::default(),
            value: Default::default(),
            read: Default::default(),
        }
    }
}

impl SingleCash {
    fn get<F>(&mut self, key: &str, f: F) -> &[String]
    where
        F: FnOnce() -> Vec<String>,
    {
        self.read = true;
        if !(self.last_key.as_ref().map(|f| f.as_str()) == Some(&key)) {
            self.last_key = Some(key.to_string());
            self.value = f();
        }

        &self.value
    }

    fn update(&mut self) {
        if !self.read {
            self.last_key = None;
            self.value.clear();
        }
    }
}

#[derive(Debug)]
pub struct FromBindings {
    editing_states: HashMap<(u8, Button), EditingStates>,
    button: u8,
    controller: u8,
    bindings: BTreeSet<(u8, Button)>,
    filtered_commands: SingleCash,
}

impl Default for FromBindings {
    fn default() -> Self {
        Self {
            editing_states: HashMap::new(),
            button: 0,
            controller: 0,
            bindings: BTreeSet::new(),
            filtered_commands: Default::default(),
        }
    }
}

impl FromBindings {
    pub fn ui(ui: &mut Ui, view: &mut Views) -> bool {
        let mut update = false;

        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("controller");
                ui.add(DragValue::new(&mut view.from_bindings.controller));
                ui.label("button");
                ui.add(DragValue::new(&mut view.from_bindings.button));
                if ui.button("add button").clicked() {
                    view.from_bindings.bindings.insert((
                        view.from_bindings.controller,
                        Button::Button(view.from_bindings.button),
                    ));
                }
            });

            view.from_bindings
                .bindings
                .retain(|b| !view.bindings.binding_to_command.contains_key(b));

            for (controller, button) in &view.from_bindings.bindings {
                ui.horizontal(|ui| {
                    ui.label(format!("{controller}:{button}"));
                    let mut vec = Vec::new();
                    Self::add_widgets(
                        ui,
                        &mut view.bindings.command_to_bindings,
                        &mut vec,
                        &view.commands,
                        view.from_bindings
                            .editing_states
                            .entry((*controller, *button))
                            .or_insert(EditingStates::default()),
                        (*controller, *button),
                        &mut view.from_bindings.filtered_commands,
                        &mut view.error,
                    );

                    if !vec.is_empty() {
                        view.bindings.binding_to_command.insert((*controller, *button), vec);
                    }
                });
            }

            for ((controller, button), commands) in &mut view.bindings.binding_to_command {
                ui.horizontal(|ui| {
                    ui.label(format!("{controller}:{button}"));

                    commands.retain(|(command, when)| {
                        ui.label(format!("{command}:{when}"));

                        let keep = !ui.button("X").clicked();

                        let b = Binding {
                            controller: *controller,
                            button: *button,
                            when: *when,
                        };

                        if !keep {
                            view.bindings
                                .command_to_bindings
                                .get_mut(command)
                                .unwrap()
                                .retain(|binding| !(*binding == b));
                        }

                        keep
                    });

                    update |= Self::add_widgets(
                        ui,
                        &mut view.bindings.command_to_bindings,
                        commands,
                        &view.commands,
                        view.from_bindings
                            .editing_states
                            .entry((*controller, *button))
                            .or_insert(EditingStates::default()),
                        (*controller, *button),
                        &mut view.from_bindings.filtered_commands,
                        &mut view.error,
                    );
                });
            }
        });

        view.from_bindings.filtered_commands.update();

        update
    }

    fn add_widgets(
        ui: &mut Ui,
        c2b: &mut BTreeMap<String, Vec<Binding>>,
        commands: &mut Vec<(String, RunWhen)>,
        allowed: &BTreeSet<String>,
        state: &mut EditingStates,
        binding: (u8, Button),
        cache: &mut SingleCash,
        errors: &mut Vec<String>,
    ) -> bool {
        ui.label("when");
        let selected = &mut state.when;

        ui.label("command");

        let edit = ui.text_edit_singleline(&mut state.command);

        let id = ui.make_persistent_id("completion box command from bindings");

        if edit.gained_focus() {
            ui.memory_mut(|mem| mem.open_popup(id));
        }

        popup_below_widget(
            ui,
            id,
            &edit,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                for command in cache.get(&state.command, || {
                    allowed
                        .iter()
                        .filter(|s| s.contains(&state.command))
                        .cloned()
                        .collect::<Vec<_>>()
                }) {
                    if ui.button(command).clicked() {
                        state.command = command.clone();
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
            },
        );

        ui.push_id(binding, |ui| {
            ComboBox::from_label("")
                .selected_text(format!("{}", selected))
                .show_ui(ui, |ui| {
                    for i in [
                        RunWhen::OnTrue,
                        RunWhen::OnFalse,
                        RunWhen::WhileTrue,
                        RunWhen::WhileFalse,
                    ] {
                        ui.selectable_value(selected, i, i.get_str());
                    }
                });
        });

        if ui.button("add").clicked() {
            if !allowed.contains(&state.command) {
                errors.push("not a valid command".to_string());
                return false;
            }

            let binding = Binding {
                controller: binding.0,
                button: binding.1,
                when: *selected,
            };

            if c2b
                .get(&state.command)
                .map_or(false, |bindings| bindings.contains(&binding))
            {
                errors.push("binding already exists".to_string());
                return false;
            }

            c2b.entry(state.command.clone())
                .or_insert(Vec::new())
                .push(binding);

            commands.push((state.command.clone(), *selected));

            true
        } else {
            false
        }
    }
}
