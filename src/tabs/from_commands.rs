use bumpalo::Bump;
use egui::{Color32, Grid, ScrollArea, Ui};

use std::{collections::HashMap, rc::Rc};

use crate::{
    bindings::{Binding, Button, RunWhen},
    component::Component,
    global_state::GlobalEvents,
    search_selector::{search_selector, SelectorCache},
    State,
};

#[derive(Debug)]
pub struct BindingEditingState {
    controller: u8,
    button: Button,
    filter: String,
    cache: SelectorCache<Button>,
    when: RunWhen,

    controller_filter: String,
    controller_cache: SelectorCache<u8>,
}

impl Default for BindingEditingState {
    fn default() -> Self {
        Self {
            controller: Default::default(),
            button: Button {
                button: 1,
                location: crate::bindings::ButtonLocation::Button,
            },
            when: RunWhen::WhileTrue,
            filter: Default::default(),
            cache: Default::default(),
            controller_filter: Default::default(),
            controller_cache: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct FromCommands {
    pub editing_states: HashMap<Rc<String>, BindingEditingState>,
}

impl Component for FromCommands {
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
            // TODO ADD POV BINDING

            Grid::new("from_commands_grid").show(ui, |ui| {
                for command in &env.commands {
                    ui.horizontal(|ui| {
                        ui.label(
                            bumpalo::format!(in arena, "{} has bindings", command.as_str())
                                .as_str(),
                        );

                        for binding in env.bindings.bindings_for_command(command) {
                            if !env.controllers[binding.controller as usize]
                                .valid_binding(binding.button)
                            {
                                ui.colored_label(
                                    Color32::from_rgb(0xf3, 0x8b, 0xa8),
                                    binding.show(env, arena),
                                );
                            } else {
                                ui.label(binding.show(env, arena));
                            }

                            if ui.button("X").clicked() {
                                output.add_event(GlobalEvents::RemoveBinding(
                                    binding,
                                    command.clone(),
                                ));
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        let edit_state = self.editing_states.entry(command.clone()).or_default();

                        ui.label("controller");

                        search_selector(
                            ui.make_persistent_id(("from commands controller", command)),
                            &mut edit_state.controller_filter,
                            &mut edit_state.controller,
                            env.controllers.iter().enumerate().flat_map(|(id, c)| {
                                if c.bound() {
                                    Some((env.controller_name(id as u8), id as u8))
                                } else {
                                    None
                                }
                            }),
                            &mut edit_state.controller_cache,
                            100.0,
                            ui,
                        );

                        ui.label("button");

                        env.controllers[edit_state.controller as usize].show_button_selector(
                            ui.make_persistent_id(("from commands button", command)),
                            &mut edit_state.filter,
                            &mut edit_state.cache,
                            &mut edit_state.button,
                            ui,
                            arena,
                        );

                        let run_when = &mut edit_state.when;

                        run_when.selection_ui(ui, command);

                        let binding = Binding {
                            controller: edit_state.controller,
                            button: edit_state.button,
                            during: edit_state.when,
                        };

                        if ui.button("add").clicked()
                            && (env.controllers[edit_state.controller as usize]
                                .valid_binding(edit_state.button))
                        {
                            output.add_event(GlobalEvents::AddBinding(binding, command.clone()));
                        }
                    });

                    ui.end_row();
                }
            });
        });
    }
}
