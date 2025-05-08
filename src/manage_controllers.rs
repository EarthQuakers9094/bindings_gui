use egui::{DragValue, ScrollArea, TextEdit};

use crate::{
    bindings::ControllerType,
    component::Component,
    global_state::{GlobalEvents, State},
};

#[derive(Debug, Default)]
pub struct ManageControllers {}

impl Component for ManageControllers {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.label("controller slots");

            for (id, controller) in env.controllers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("slot {id}"));

                    ui.label("name: ");

                    if controller.bound() {
                        TextEdit::singleline(&mut env.controller_names[id as usize])
                            .desired_width(100.0)
                            .show(ui);
                    }

                    match controller {
                        ControllerType::NotBound => {
                            ui.horizontal(|ui| {
                                if ui.button("set xbox").clicked() {
                                    *controller = ControllerType::XBox { sensitivity: 0.5 };
                                    output.add_event(GlobalEvents::Save);
                                }
                                if ui.button("set generic").clicked() {
                                    *controller = ControllerType::Generic { buttons: 0 };
                                    output.add_event(GlobalEvents::Save);
                                }
                            });
                        }
                        ControllerType::Generic { buttons } => {
                            ui.label("generic");

                            ui.label("buttons: ");

                            let b = *buttons;

                            ui.add(DragValue::new(buttons).range(0..=32));

                            if b != *buttons {
                                output.add_event(GlobalEvents::Save);
                            }

                            if ui.button("remove").clicked() {
                                *controller = ControllerType::NotBound;
                                output.add_event(GlobalEvents::Save);
                            }
                        }
                        ControllerType::XBox { sensitivity } => {
                                ui.label("xbox");

                                ui.label("trigger sensitivity");

                                ui.add(DragValue::new(sensitivity).range(0..=1).speed(0.1));

                                if ui.button("remove").clicked() {
                                    *controller = ControllerType::NotBound;
                                    output.add_event(GlobalEvents::Save);
                                }
                        }
                    };
                });
            }
        });
    }
}
