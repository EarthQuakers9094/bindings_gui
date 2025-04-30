use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
};

use egui::{ScrollArea, Ui};

use crate::{Binding, Views};

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

                let mut to_remove = Vec::new();

                for command in view.commands.iter() {
                    ui.horizontal(|ui| {
                        ui.label(command);
                        if ui.button("X").clicked() {
                            let valid_remove = view
                                .command_to_bindings
                                .get(command)
                                .map(|n| n.is_empty())
                                .unwrap_or(true);

                            if valid_remove {
                                to_remove.push(command.clone()); // TODO try to remove this clone
                            } else {
                                todo!("display that can't remove because still used")
                            }
                        }
                    });
                }

                update |= !to_remove.is_empty();

                for c in to_remove.iter() {
                    view.commands.remove(c.deref());
                }

                update
            })
            .inner
    }
}
