use egui::{DragValue, ScrollArea};

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
        env: &Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            for (id, controller) in env.controllers.iter().enumerate() {
                match controller {
                    ControllerType::NotBound => {
                        ui.horizontal(|ui| {
                            if ui.button("set xbox").clicked() {
                                output.add_event(GlobalEvents::BindController(
                                    ControllerType::XBox,
                                    id as u8,
                                ));
                            }
                            if ui.button("set generic").clicked() {
                                output.add_event(GlobalEvents::BindController(
                                    ControllerType::Generic { buttons: 0 },
                                    id as u8,
                                ));
                            }
                        });
                    }
                    ControllerType::Generic { buttons } => {
                        let mut bs = *buttons;

                        ui.horizontal(|ui| {
                            ui.label("buttons: ");

                            ui.add(DragValue::new(&mut bs).range(0..=32));

                            if bs != *buttons {
                                output.add_event(GlobalEvents::BindController(
                                    ControllerType::Generic { buttons: bs },
                                    id as u8,
                                ));
                            }

                            if ui.button("remove").clicked() {
                                output.add_event(GlobalEvents::BindController(
                                    ControllerType::NotBound,
                                    id as u8,
                                ));
                            }
                        });
                    }
                    ControllerType::XBox => {
                        ui.horizontal(|ui| {
                            ui.label("xbox");
                            if ui.button("remove").clicked() {
                                output.add_event(GlobalEvents::BindController(
                                    ControllerType::NotBound,
                                    id as u8,
                                ));
                            }
                        });
                    }
                };
            }
        });
    }
}
