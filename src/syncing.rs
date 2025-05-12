use egui::TextEdit;

use crate::{
    global_state::{GlobalEvents, State},
    Component,
};

#[derive(Debug)]
pub struct SyncingTab {}

impl Component for SyncingTab {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
        arena: &bumpalo::Bump,
    ) {
        match &mut env.url {
            Some(url) => {
                let before = bumpalo::collections::String::from_str_in(url, arena);

                ui.horizontal(|ui| {
                    ui.label("url: ");
                    TextEdit::singleline(url).show(ui);
                });

                if before.as_str() != url {
                    output.add_event(GlobalEvents::Save);
                }

                if env.syncing {
                    if ui.button("disable syncing").clicked() {
                        env.syncing = false;
                    }
                } else if ui.button("enable syncing").clicked() {
                    env.syncing = true;
                }
            }
            None => {
                if ui.button("setup syncing").clicked() {
                    env.url = Some("".to_string());
                }
            }
        }
    }
}
