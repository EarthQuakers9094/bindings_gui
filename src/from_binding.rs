use std::{
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

use bumpalo::Bump;
use egui::{Color32, ScrollArea, Ui};

use crate::{
    bindings::{Binding, Button, RunWhen},
    component::{Component, EventStream},
    global_state::GlobalEvents,
    search_selector::{search_selector, SingleCache},
    State,
};

#[derive(Debug)]
pub struct EditingStates {
    command: Rc<String>,
    filter: String,
    when: RunWhen,
}

impl Default for EditingStates {
    fn default() -> Self {
        Self {
            command: Rc::new("".to_string()),
            when: RunWhen::WhileTrue,
            filter: "".to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub struct FromBindings {
    pub editing_states: HashMap<(u8, Button), EditingStates>,
    pub button: Button,
    pub controller: u8,
    pub bindings: BTreeSet<(u8, Button)>,
    pub button_filter: String,
    pub button_filter_cache: SingleCache<String, Vec<(Rc<String>, Button)>>,
    pub filtered_commands: SingleCache<String, Vec<(Rc<String>, Rc<String>)>>,
    pub controller_filter: String,
    pub controller_cache: SingleCache<String, Vec<(Rc<String>, u8)>>,
}

impl Component for FromBindings {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut Ui,
        env: &mut Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
        arena: &Bump,
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("controller");

                search_selector(
                    ui.make_persistent_id("controller_selector"),
                    &mut self.controller_filter,
                    &mut self.controller,
                    env.controllers.iter().enumerate().flat_map(|(id, c)| {
                        if c.bound() {
                            Some((env.controller_name(id as u8), id as u8))
                        } else {
                            None
                        }
                    }),
                    &mut self.controller_cache,
                    100.0,
                    ui,
                );

                self.controller_cache.update();

                ui.label("button");

                env.controllers[self.controller as usize].show_button_selector(
                    ui.make_persistent_id("bindings button selector"),
                    &mut self.button_filter,
                    &mut self.button_filter_cache,
                    &mut self.button,
                    ui,
                    arena,
                );

                self.button_filter_cache.update();

                if ui.button("add button").clicked()
                    && env.valid_binding(self.controller, self.button)
                {
                    self.bindings.insert((self.controller, self.button));
                }
            });

            ui.separator();

            self.bindings.retain(|b| !env.bindings.has_button(*b));

            egui::Grid::new("from_bindings_grid").show(ui, |ui| {
                for (controller, button) in &self.bindings {
                    Self::display_binding(*controller, *button, env, ui, arena);

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
                    Self::display_binding(*controller, *button, env, ui, arena);

                    ui.horizontal(|ui| {
                        for (command, when) in commands {
                            ui.label(bumpalo::format!(in &arena, "{} {}", command, when).as_str());

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
    fn display_binding(controller: u8, button: Button, env: &State, ui: &mut Ui, arena: &Bump) {
        let text = bumpalo::format!(in &arena,
            "{} {} has bindings",
            env.controller_name(controller),
            env.controllers[controller as usize].button_name(&button, arena)
        );

        if env.valid_binding(controller, button) {
            ui.label(text.as_str())
        } else {
            ui.colored_label(Color32::from_rgb(0xf3, 0x8b, 0xa8), text.as_str())
        };
    }

    fn add_widgets(
        cache: &mut SingleCache<String, Vec<(Rc<String>, Rc<String>)>>,
        ui: &mut Ui,
        env: &State,
        output: &EventStream<GlobalEvents>,
        state: &mut EditingStates,
        binding: (u8, Button),
    ) {
        ui.horizontal(|ui| {
            ui.label("command");

            search_selector(
                ui.make_persistent_id(("command selector from bindings", binding)),
                &mut state.filter,
                &mut state.command,
                env.commands.iter().map(|a| (a.clone(), a.clone())),
                cache,
                200.0,
                ui,
            );

            ui.label("when");
            let when_run: &mut RunWhen = &mut state.when;

            when_run.selection_ui(ui, binding);

            if ui.button("add").clicked() {
                if !env.commands.contains(&state.command) {
                    output.add_event(GlobalEvents::DisplayError(
                        "not a valid command (maybe try adding it in manage commands)".to_string(),
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
