use egui::{ScrollArea, Ui};

use crate::Views;

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

impl ManageTab {
    pub fn ui(
        ui: &mut Ui,
        view: &mut Views,
    ) -> bool {
        ScrollArea::vertical()
            .show(ui, |ui| {
                // TODO ADD RENAME FUNCTIONALITY

                let mut update = false;
                let adding = &mut view.manage_tab.adding;

                ui.horizontal(|ui| {
                    ui.text_edit_singleline(adding);

                    if ui.button("add").clicked() && !adding.is_empty() {
                        view.commands.insert(adding.clone());
                        *adding = "".to_string();
                        update = true;
                    }
                });

                view.commands.retain(|command| {
                    ui.horizontal(|ui| {
                        ui.label(command);
                        if ui.button("X").clicked() {
                            let valid_remove = view
                                .command_to_bindings
                                .get(command)
                                .map(|n| n.is_empty())
                                .unwrap_or(true);

                            if valid_remove {
                                update = true;
                                false
                            } else {
                                view.error.push("can't delete a command that is still used".to_string());
                                true
                            }
                        } else {
                            true
                        }
                    }).inner
                });

                update
            })
            .inner
    }
}
