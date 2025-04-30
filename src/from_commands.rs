use std::fmt::format;

use egui::{
    ahash::{HashMap, HashMapExt},
    ComboBox, ScrollArea, Ui,
};

use crate::{Binding, Button, RunWhen, Views};

#[derive(Debug)]
pub struct BindingEditingState {
    controller: u8,
    button: u8,
    when: RunWhen,
}

impl Default for BindingEditingState {
    fn default() -> Self {
        Self {
            controller: Default::default(),
            button: Default::default(),
            when: RunWhen::WhileTrue,
        }
    }
}

#[derive(Debug)]
pub struct FromCommands {
    editing_states: HashMap<String, BindingEditingState>,
}

impl Default for FromCommands {
    fn default() -> Self {
        Self {
            editing_states: HashMap::new(),
        }
    }
}

impl FromCommands {
    pub fn ui(ui: &mut Ui, view: &mut Views) -> bool {
        ScrollArea::vertical().show(ui, |ui| {
            for command in view.commands.iter() {
                let v: Vec<_> = Vec::new();

                ui.horizontal(|ui| {
                    ui.label(command);
                    let bindings = view.command_to_bindings.get(command).unwrap_or(&v);

                    for b in bindings {
                        ui.label(format!("{b}"));
                    }
                    
                    let edit_state = view
                        .from_commands
                        .editing_states
                        .entry(command.clone())
                        .or_insert(BindingEditingState::default());

                    ui.label("controller");

                    ui.add(egui::DragValue::new(&mut edit_state.controller));

                    ui.label("button");

                    ui.add(egui::DragValue::new(&mut edit_state.button));

                    let selected = &mut edit_state.when;

                    ui.push_id(command, |ui| {
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

                    let binding = Binding {
                        controller: edit_state.controller,
                        button: Button::Button(edit_state.button),
                        when: edit_state.when,
                    };

                    if ui.button("add").clicked() {
                        view.command_to_bindings
                            .entry(command.clone())
                            .or_insert(Vec::new())
                            .push(binding);

                        view.binding_to_command
                            .entry(binding)
                            .or_insert(Vec::new())
                            .push(command.clone());
                    }
                });
            }
        });

        false
    }
}
