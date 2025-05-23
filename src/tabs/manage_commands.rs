use std::{collections::HashMap, rc::Rc};

use bumpalo::Bump;
use egui::{ScrollArea, TextEdit, Ui};

use crate::{component::Component, global_state::GlobalEvents, State};

#[derive(Debug)]
pub(crate) struct ManageTab {
    pub adding: String,
    pub rename: HashMap<Rc<String>, String>,
}

impl Default for ManageTab {
    fn default() -> Self {
        Self {
            adding: "".to_string(),
            rename: HashMap::new(),
        }
    }
}

impl Component for ManageTab {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut Ui,
        env: &mut Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
        _arena: &Bump,
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            let mut update = false;
            let adding = &mut self.adding;

            ui.horizontal(|ui| {
                ui.text_edit_singleline(adding);

                if ui.button("add").clicked() && !adding.is_empty() {
                    output.add_event(GlobalEvents::AddCommand(std::mem::take(adding)));
                    update = true;
                }
            });

            ui.separator();

            for command in &env.commands {
                ui.horizontal(|ui| {
                    let rename = self
                        .rename
                        .entry(command.clone())
                        .or_insert_with(|| command.to_string());

                    let resp = ui.add(
                        TextEdit::singleline(rename)
                            .frame(false)
                            .desired_width(100.0),
                    );

                    if resp.lost_focus() {
                        if env.commands.contains(rename) {
                            output.add_event(GlobalEvents::DisplayError(
                                "command of that name already exists".to_string(),
                            ));
                        } else {
                            output.add_event(GlobalEvents::RenameCommand(
                                command.clone(),
                                Rc::new(rename.clone()),
                            ));
                        }
                    }

                    if ui.button("X").clicked() {
                        let valid_remove = !env.bindings.is_used(command);

                        if valid_remove {
                            output.add_event(GlobalEvents::RemoveCommand(command.clone()));
                        } else {
                            output.add_event(GlobalEvents::DisplayError(
                                "can't delete a command that is still used".to_string(),
                            ));
                        }
                    }
                });
            }
        });
    }
}
