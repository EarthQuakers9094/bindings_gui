use egui::{ScrollArea, Ui};

use std::collections::HashMap;

use crate::{
    bindings::{Binding, Button, RunWhen},
    component::Compenent,
    global_state::GlobalEvents,
    State,
};

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

#[derive(Debug, Default)]
pub struct FromCommands {
    pub editing_states: HashMap<String, BindingEditingState>,
}


impl Compenent for FromCommands {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut Ui,
        env: &Self::Environment,
        output: &mut crate::component::EventStream<Self::OutputEvents>,
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            // TODO ADD POV BINDING

            for command in &env.commands {
                ui.horizontal(|ui| {
                    ui.label(command);

                    for binding in env
                        .bindings
                        .command_to_bindings
                        .get(command)
                        .unwrap_or(&Vec::new())
                    {
                        ui.label(binding.to_string());

                        if !ui.button("X").clicked() {
                            output
                                .add_event(GlobalEvents::RemoveBinding(*binding, command.clone()));
                        }
                    }

                    let edit_state = self.editing_states.entry(command.clone()).or_default();

                    ui.label("controller");

                    ui.add(egui::DragValue::new(&mut edit_state.controller));

                    ui.label("button");

                    ui.add(egui::DragValue::new(&mut edit_state.button));

                    let run_when = &mut edit_state.when;

                    run_when.selection_ui(ui, command);

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
