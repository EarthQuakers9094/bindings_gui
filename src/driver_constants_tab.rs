use std::{collections::BTreeMap, rc::Rc};

use bumpalo::Bump;
use egui::{collapsing_header::CollapsingState, DragValue, ScrollArea, Ui};

use crate::{
    component::EventStream,
    constants::{Constants, OptionLocation},
    global_state::{GlobalEvents, State},
    Component,
};

#[derive(Debug, Default)]
pub struct DriverConstantsTab {}

impl Component for DriverConstantsTab {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
        arena: &bumpalo::Bump,
    ) {
        let mut modified = false;

        ScrollArea::vertical().show(ui, |ui| {
            let constants = &mut env.driver_constants;

            match &env.constants {
                Constants::Object { map } => {
                    for (key, value) in map.iter() {
                        match value {
                            Constants::Object { map } => {
                                modified |= self.show_object(
                                    key.clone(),
                                    map,
                                    constants
                                        .make_object_mut()
                                        .entry(key.clone())
                                        .or_insert(Constants::Object {
                                            map: BTreeMap::new(),
                                        })
                                        .make_object_mut(),
                                    Rc::new(Vec::new()),
                                    output,
                                    arena,
                                    ui,
                                );
                            }

                            Constants::Driver { default } => {
                                modified |= Self::show_value(
                                    key.clone(),
                                    constants
                                        .make_object_mut()
                                        .entry(key.clone())
                                        .or_insert(Constants::None)
                                        .make_mut(default),
                                    &default,
                                    ui,
                                    arena,
                                );
                            }

                            _ => {}
                        }
                    }
                }
                Constants::None => {}
                _ => {
                    ui.label("shouldn't have constants at base level");
                }
            }
        });

        if modified {
            output.add_event(GlobalEvents::Save);
        }
    }
}

impl DriverConstantsTab {
    fn show_object(
        &mut self,
        name: Rc<String>,
        map: &BTreeMap<Rc<String>, Constants>,
        constants: &mut BTreeMap<Rc<String>, Constants>,
        mut key_path: OptionLocation,
        output: &EventStream<GlobalEvents>,
        arena: &Bump,
        ui: &mut Ui,
    ) -> bool {
        let mut modified = false;

        let k = Rc::make_mut(&mut key_path);

        k.push(name.clone());

        CollapsingState::load_with_default_open(
            ui.ctx(),
            ui.make_persistent_id("object header"),
            false,
        )
        .show_header(ui, |ui| {
            ui.label(name.as_str());
        })
        .body(|ui| {
            for (key, value) in map {
                match value {
                    Constants::Object { map } => {
                        modified |= self.show_object(
                            key.clone(),
                            map,
                            constants
                                .entry(key.clone())
                                .or_insert(Constants::Object {
                                    map: BTreeMap::new(),
                                })
                                .make_object_mut(),
                            key_path.clone(),
                            output,
                            arena,
                            ui,
                        );
                    }

                    Constants::Driver { default } => {
                        modified |= Self::show_value(
                            key.clone(),
                            constants
                                .entry(key.clone())
                                .or_insert(Constants::None)
                                .make_mut(default),
                            &default,
                            ui,
                            arena,
                        );
                    }

                    _ => {}
                }
            }
        });

        modified
    }

    fn show_value(
        name: Rc<String>,
        constant: &mut Constants,
        default: &Constants,
        ui: &mut Ui,
        arena: &Bump,
    ) -> bool {
        ui.horizontal(|ui| {
            ui.label(bumpalo::format!(in &arena, "{} = ", name).as_str());
            let ret = Self::modify_value(constant, ui);

            if ui.button("reset").clicked() {
                *constant = default.clone();
            }

            ret
        })
        .inner
    }

    fn modify_value(constant: &mut Constants, ui: &mut Ui) -> bool {
        match constant {
            Constants::Object { .. } => panic!("invalid argument"),
            Constants::Float(f) => {
                let o = *f;
                ui.add(DragValue::new(f).speed(0.01));

                *f != o
            }
            Constants::Int(i) => {
                let o = *i;
                ui.add(DragValue::new(i));

                *i != o
            }
            Constants::String(s) => ui.text_edit_singleline(s).lost_focus(),
            Constants::Driver { default } => {
                ui.label("default");
                Self::modify_value(default.as_mut(), ui)
            }
            Constants::None => {
                ui.label("null");
                false
            }
        }
    }
}
