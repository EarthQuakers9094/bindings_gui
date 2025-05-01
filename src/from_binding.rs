use egui::{
    ahash::{HashMap, HashMapExt}, ComboBox, DragValue, ScrollArea, Slider, Ui
};

use crate::{Binding, Button, RunWhen, Views};

#[derive(Debug)]
pub struct CommandEditingStates {
    command: String,
    when: RunWhen,
}

impl Default for CommandEditingStates {
    fn default() -> Self {
        Self {
            command: "".to_string(),
            when: RunWhen::WhileTrue,
        }
    }
}

#[derive(Debug)]
pub struct FromBindings {
    editing_states: HashMap<String, CommandEditingStates>,
    button: u8,
    controller: u8, 
}

impl Default for FromBindings {
    fn default() -> Self {
        Self {
            editing_states: HashMap::new(),
            button: 0,
            controller: 0,
        }
    }
}

impl FromBindings {
    pub fn ui(ui: &mut Ui, view: &mut Views) -> bool {
        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("controller");
                ui.add(DragValue::new(&mut view.from_bindings.controller));
                ui.label("button");
                ui.add(DragValue::new(&mut view.from_bindings.button));
                if ui.button("add button").clicked() {
                    // view.add_binding();
                }
            })

        });

        // view.binding_to_command

        // ScrollArea::vertical().show(ui, |ui| {
        //     for command in view.commands.iter() {
        //         ui.horizontal(|ui| {
        //             ui.label(command);

        //             if let Some(bindings) = view.command_to_bindings.get_mut(command) {
        //                 bindings.retain(|b| {
        //                     ui.label(format!("{b}"));

        //                     !ui.button("X").clicked()
        //                 });
        //             }

        //             let edit_state = view
        //                 .from_commands
        //                 .editing_states
        //                 .entry(command.clone())
        //                 .or_insert(CommandEditingStates::default());

        //             ui.label("controller");

        //             ui.add(egui::DragValue::new(&mut edit_state.controller));

        //             ui.label("button");

        //             ui.add(egui::DragValue::new(&mut edit_state.button));

        //             let selected = &mut edit_state.when;

        //             ui.push_id(command, |ui| {
        //                 ComboBox::from_label("")
        //                     .selected_text(format!("{}", selected))
        //                     .show_ui(ui, |ui| {
        //                         for i in [
        //                             RunWhen::OnTrue,
        //                             RunWhen::OnFalse,
        //                             RunWhen::WhileTrue,
        //                             RunWhen::WhileFalse,
        //                         ] {
        //                             ui.selectable_value(selected, i, i.get_str());
        //                         }
        //                     });
        //             });

        //             let binding = Binding {
        //                 controller: edit_state.controller,
        //                 button: Button::Button(edit_state.button),
        //                 when: edit_state.when,
        //             };

        //             if ui.button("add").clicked() {
        //                 if view
        //                     .command_to_bindings
        //                     .get(command)
        //                     .unwrap_or(&Vec::new())
        //                     .contains(&binding)
        //                 {
        //                     view.error.push("you already have this binding".to_string());
        //                 } else {
        //                     view.command_to_bindings
        //                         .entry(command.clone())
        //                         .or_insert(Vec::new())
        //                         .push(binding);

        //                     view.binding_to_command
        //                         .entry(binding)
        //                         .or_insert(Vec::new())
        //                         .push(command.clone());
        //                 }
        //             }
        //         });
        //     }
        // });

        false
    }
}
