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
            // TODO ADD POV BINDING

            for command in &view.commands {
                ui.horizontal(|ui| {
                    ui.label(command);

                    view.bindings.retain_if_command(command, |binding| {
                        ui.label(binding.to_string());
                        
                        !ui.button("X").clicked()
                    });

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
                        view.bindings.try_add_binding(command, binding, &mut view.error);                        
                    }
                });
            }
        });

        false
    }
}
