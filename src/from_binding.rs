use std::collections::{BTreeSet, HashMap};

use egui::{popup_below_widget, DragValue, ScrollArea, Ui};

use crate::{
    bindings::{Binding, Button, RunWhen},
    component::{Compenent, EventStream},
    global_state::GlobalEvents,
    State,
};

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

#[derive(Debug, Default)]
pub struct SingleCash {
    last_key: Option<String>,
    value: Vec<String>,
    read: bool,
}

impl SingleCash {
    fn get<F>(&mut self, key: &str, f: F) -> &[String]
    where
        F: FnOnce() -> Vec<String>,
    {
        self.read = true;
        if self.last_key.as_deref() != Some(key) {
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

#[derive(Debug, Default)]
pub struct FromBindings {
    pub editing_states: HashMap<(u8, Button), EditingStates>,
    pub button: u8,
    pub controller: u8,
    pub bindings: BTreeSet<(u8, Button)>,
    pub filtered_commands: SingleCash,
}

impl Compenent for FromBindings {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut Ui,
        env: &Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("controller");
                ui.add(DragValue::new(&mut self.controller));
                ui.label("button");
                ui.add(DragValue::new(&mut self.button));
                if ui.button("add button").clicked() {
                    self.bindings.insert((
                        self.controller,
                        Button {
                            button: self.button as i16,
                            location: crate::bindings::ButtonLocation::Button,
                        },
                    ));
                }
            });

            self.bindings.retain(|b| !env.bindings.has_button(*b));

            egui::Grid::new("from_bindings_grid").show(ui, |ui| {
                for (controller, button) in &self.bindings {
                    ui.label(format!("{controller}:{button}"));

                    Self::add_widgets(
                        &mut self.filtered_commands,
                        ui,
                        env,
                        output,
                        self.editing_states
                            .entry((*controller, *button))
                            .or_default(),
                        (*controller, *button),
                    );

                    ui.end_row();
                }

                for ((controller, button), commands) in &env.bindings.binding_to_commands {
                    ui.horizontal(|ui| {
                        ui.label(format!("{controller}:{button}"));

                        for (command, when) in commands {
                            ui.label(format!("{command}:{when}"));

                            let keep = !ui.button("X").clicked();

                            if !keep {
                                output.add_event(GlobalEvents::RemoveBinding(
                                    Binding {
                                        controller: *controller,
                                        button: *button,
                                        during: *when,
                                    },
                                    command.clone(),
                                ));
                            }
                        }
                    });

                    Self::add_widgets(
                        &mut self.filtered_commands,
                        ui,
                        env,
                        output,
                        self.editing_states
                            .entry((*controller, *button))
                            .or_default(),
                        (*controller, *button),
                    );

                    ui.end_row();
                }
            });
        });

        self.filtered_commands.update();
    }
}

impl FromBindings {
    fn add_widgets(
        cache: &mut SingleCash,
        ui: &mut Ui,
        env: &State,
        output: &EventStream<GlobalEvents>,
        state: &mut EditingStates,
        binding: (u8, Button),
    ) {
        ui.horizontal(|ui| {
            ui.label("when");
            let when_run = &mut state.when;

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
                        env.commands
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

            when_run.selection_ui(ui, binding);

            if ui.button("add").clicked() {
                if !env.commands.contains(&state.command) {
                    output.add_event(GlobalEvents::DisplayError(
                        "not a valid command".to_string(),
                    ));
                    return;
                }

                let binding = Binding {
                    controller: binding.0,
                    button: binding.1,
                    during: *when_run,
                };

                if env.bindings.has_binding(&state.command, binding) {
                    output.add_event(GlobalEvents::DisplayError(
                        "binding already exists".to_string(),
                    ));
                    return;
                }

                output.add_event(GlobalEvents::AddBinding(binding, state.command.clone()));
            }
        });
    }
}
