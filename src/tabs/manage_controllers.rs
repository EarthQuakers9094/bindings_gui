use std::rc::Rc;

use bumpalo::Bump;
use egui::{DragValue, ScrollArea, TextEdit};

use crate::{
    bindings::ControllerType,
    component::Component,
    global_state::{GlobalEvents, State},
};

#[derive(Debug, Default, Clone)]
pub struct ManageControllers {}

impl Component for ManageControllers {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
        arena: &Bump,
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.label("controller slots");
            ui.separator();

            for (id, controller) in env.controllers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(bumpalo::format!(in &arena, "slot {}", id).as_str());

                    ui.label("name: ");

                    if controller.bound() {
                        let before = bumpalo::collections::String::from_str_in(
                            &env.controller_names[id],
                            arena,
                        );

                        let s = Rc::make_mut(&mut env.controller_names[id]);

                        TextEdit::singleline(s).desired_width(100.0).show(ui);

                        if before != s.as_str() {
                            output.add_event(GlobalEvents::Save);
                        }
                    }

                    match controller {
                        ControllerType::NotBound => {
                            ui.horizontal(|ui| {
                                if ui.button("set xbox").clicked() {
                                    *controller = ControllerType::XBox { sensitivity: 0.5 };
                                    output.add_event(GlobalEvents::Save);
                                }
                                if ui.button("set generic").clicked() {
                                    *controller = ControllerType::Generic {
                                        buttons: 0,
                                        axises: 0,
                                    };
                                    output.add_event(GlobalEvents::Save);
                                }
                            });
                        }
                        ControllerType::Generic { buttons, axises } => {
                            ui.label("generic");

                            ui.label("buttons: ");

                            let b = *buttons;

                            ui.add(DragValue::new(buttons).range(0..=32));

                            let a = *axises;

                            ui.label("axises: ");

                            ui.add(DragValue::new(axises).range(0..=32));

                            if b != *buttons || a != *axises {
                                output.add_event(GlobalEvents::Save);
                            }

                            if ui.button("remove").clicked() {
                                *controller = ControllerType::NotBound;
                                output.add_event(GlobalEvents::Save);
                            }
                        }
                        ControllerType::XBox { sensitivity } => {
                            ui.label("xbox");

                            ui.label("trigger sensitivity (can't live reload)");

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

    fn tab_type(&self) -> super::TabType {
        super::TabType::ManageControllers
    }
}
