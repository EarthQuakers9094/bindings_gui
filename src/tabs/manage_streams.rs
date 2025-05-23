use std::{collections::HashMap, rc::Rc};

use bumpalo::Bump;
use egui::{ScrollArea, TextEdit, Ui};

use crate::{component::Component, global_state::GlobalEvents, State};

#[derive(Debug, Clone)]
pub(crate) struct ManageStreamsTab {
    pub adding: String,
    pub rename: HashMap<Rc<String>, String>,
}

impl Default for ManageStreamsTab {
    fn default() -> Self {
        Self {
            adding: "".to_string(),
            rename: HashMap::new(),
        }
    }
}

impl Component for ManageStreamsTab {
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
                    output.add_event(GlobalEvents::AddStream(std::mem::take(adding)));
                    update = true;
                }
            });

            ui.separator();

            for stream in &env.streams {
                ui.horizontal(|ui| {
                    let rename = self
                        .rename
                        .entry(stream.clone())
                        .or_insert_with(|| stream.to_string());

                    let resp = ui.add(
                        TextEdit::singleline(rename)
                            .frame(false)
                            .desired_width(100.0),
                    );

                    if resp.lost_focus() {
                        if env.streams.contains(rename) {
                            output.add_event(GlobalEvents::DisplayError(
                                "stream of that name already exists".to_string(),
                            ));
                        } else {
                            output.add_event(GlobalEvents::RenameStream(
                                stream.clone(),
                                Rc::new(rename.clone()),
                            ));
                        }
                    }

                    if ui.button("X").clicked() {
                        let is_used = match env.is_stream_used(stream) {
                            Ok(a) => a,
                            Err(err) => {
                                output.add_event(GlobalEvents::DisplayError(err.to_string()));
                                true
                            }
                        };

                        let valid_remove = !is_used;

                        if valid_remove {
                            output.add_event(GlobalEvents::RemoveStream(stream.clone()));
                        } else {
                            output.add_event(GlobalEvents::DisplayError(
                                "can't delete a stream that is still used".to_string(),
                            ));
                        }
                    }
                });
            }
        });
    }

    fn tab_type(&self) -> super::TabType {
        super::TabType::ManageSteams
    }
}
