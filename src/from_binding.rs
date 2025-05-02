use std::collections::BTreeSet;

use egui::{
    ahash::{HashMap, HashMapExt},
    popup_below_widget, ComboBox, DragValue, ScrollArea, Ui,
};

use crate::{
    component::{Compenent, EventStream},
    global_state::GlobalEvents,
    Binding, Button, RunWhen, Views,
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

#[derive(Debug)]
pub struct SingleCash {
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
    pub editing_states: HashMap<(u8, Button), EditingStates>,
    pub button: u8,
    pub controller: u8,
    pub bindings: BTreeSet<(u8, Button)>,
    pub filtered_commands: SingleCash,
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

impl Compenent for FromBindings {
    type OutputEvents = GlobalEvents;

    type Environment = Views;

    fn render(
        &mut self,
        ui: &mut Ui,
        env: &Self::Environment,
        output: &mut crate::component::EventStream<Self::OutputEvents>,
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("controller");
                ui.add(DragValue::new(&mut self.controller));
                ui.label("button");
                ui.add(DragValue::new(&mut self.button));
                if ui.button("add button").clicked() {
                    self.bindings
                        .insert((self.controller, Button::Button(self.button)));
                }
            });

            self.bindings
                .retain(|b| !env.bindings.binding_to_command.contains_key(b));

            for (controller, button) in &self.bindings {
                ui.horizontal(|ui| {
                    ui.label(format!("{controller}:{button}"));

                    Self::add_widgets(
                        &mut self.filtered_commands,
                        ui,
                        env,
                        output,
                        self.editing_states
                            .entry((*controller, *button))
                            .or_insert(EditingStates::default()),
                        (*controller, *button),
                    );

                    // Self::add_widgets(
                    //     ui,
                    //     &mut view.bindings.command_to_bindings,
                    //     &mut vec,
                    //     &view.commands,
                    //     view.from_bindings
                    //
                    //     (*controller, *button),
                    //     &mut view.from_bindings.filtered_commands,
                    //     &mut view.error,
                    // );

                    // if !vec.is_empty() {
                    //     view.bindings.binding_to_command.insert((*controller, *button), vec);
                    // }
                });
            }

            for ((controller, button), commands) in &env.bindings.binding_to_command {
                ui.horizontal(|ui| {
                    ui.label(format!("{controller}:{button}"));

                    for (command, when) in commands {
                        ui.label(format!("{command}:{when}"));

                        let keep = !ui.button("X").clicked();

                        let b = Binding {
                            controller: *controller,
                            button: *button,
                            when: *when,
                        };

                        if !keep {
                            output.add_event(GlobalEvents::RemoveBinding(
                                b,
                                command.clone(),
                            ));
                        }
                    }

                    Self::add_widgets(
                        &mut self.filtered_commands,
                        ui,
                        env,
                        output,
                        self.editing_states
                            .entry((*controller, *button))
                            .or_insert(EditingStates::default()),
                        (*controller, *button),
                    );
                });
            }
        });

        self.filtered_commands.update();
    }
}

impl FromBindings {
    // I really hate this function but I don't know how to make it not awful

    fn add_widgets(
        cache: &mut SingleCash,
        ui: &mut Ui,
        env: &Views,
        output: &mut EventStream<GlobalEvents>,
        state: &mut EditingStates,
        binding: (u8, Button),
    ) {
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
            if !env.commands.contains(&state.command) {
                output.add_event(GlobalEvents::DisplayError(
                    "not a valid command".to_string(),
                ));
                return;
            }

            let binding = Binding {
                controller: binding.0,
                button: binding.1,
                when: *selected,
            };

            if env.bindings.has_binding(&state.command, binding) {
                output.add_event(GlobalEvents::DisplayError(
                    "binding already exists".to_string(),
                ));
                return;
            }

            output.add_event(GlobalEvents::AddBinding(binding, selected.to_string()));
        }
    }
}
