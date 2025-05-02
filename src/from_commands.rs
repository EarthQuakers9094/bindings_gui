use egui::{
    ahash::{HashMap, HashMapExt},
    ComboBox, ScrollArea, Ui,
};

use crate::{component::Compenent, global_state::GlobalEvents, Binding, Button, RunWhen, Views};

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
    pub editing_states: HashMap<String, BindingEditingState>,
}

impl Default for FromCommands {
    fn default() -> Self {
        Self {
            editing_states: HashMap::new(),
        }
    }
}

impl Compenent for FromCommands {
    type OutputEvents = GlobalEvents;

    type Environment = Views;

    fn render(&mut self, ui: &mut Ui, env: &Self::Environment, output: &mut crate::component::EventStream<Self::OutputEvents>) {
        ScrollArea::vertical().show(ui, |ui| {
            // TODO ADD POV BINDING

            for command in &env.commands {
                ui.horizontal(|ui| {
                    ui.label(command);

                    for binding in env.bindings.command_to_bindings.get(command).unwrap_or(&Vec::new()) {
                        ui.label(binding.to_string());

                        if !ui.button("X").clicked() {
                            output.add_event(GlobalEvents::RemoveBinding(*binding, command.clone()));
                        }
                    }

                    let edit_state = self
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
                        output.add_event(GlobalEvents::AddBinding(binding, command.clone()));
                    }
                });
            }
        });
    }
}