use egui::{ScrollArea, Ui};

use crate::{component::Component, global_state::GlobalEvents, State};

#[derive(Debug)]
pub(crate) struct ManageTab {
    pub adding: String,
}

impl Default for ManageTab {
    fn default() -> Self {
        Self {
            adding: "".to_string(),
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
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            // TODO ADD RENAME FUNCTIONALITY

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
                    ui.label(command);
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
