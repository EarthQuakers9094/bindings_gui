use std::{collections::BTreeMap, rc::Rc};

use bumpalo::Bump;
use egui::{collapsing_header::CollapsingState, ScrollArea, Ui};

use crate::{
    component::EventStream,
    constants::Constants,
    global_state::{GlobalEvents, State},
    single_linked_list::SingleLinkedList,
    Component,
};

use super::TabType;

#[derive(Debug, Default, Clone)]
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
                        let end = SingleLinkedList::new();
                        let key_path = SingleLinkedList::Value(key.clone(), &end);

                        match value {
                            Constants::Object { map } => {
                                modified |= Self::show_object(
                                    key.clone(),
                                    map,
                                    output,
                                    constants
                                        .make_object_mut()
                                        .entry(key.clone())
                                        .or_insert(Constants::Object {
                                            map: BTreeMap::new(),
                                        })
                                        .make_object_mut(),
                                    &key_path,
                                    arena,
                                    ui,
                                );
                            }

                            Constants::Driver { default } => {
                                modified |= Self::show_value(
                                    key.clone(),
                                    &key_path,
                                    constants.make_object_mut().get_mut(key),
                                    default,
                                    output,
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

    fn tab_type(&self) -> super::TabType {
        TabType::DriverConstants
    }
}

impl DriverConstantsTab {
    fn show_object(
        name: Rc<String>,
        map: &BTreeMap<Rc<String>, Constants>,
        output: &EventStream<GlobalEvents>,
        constants: &mut BTreeMap<Rc<String>, Constants>,
        key_path: &SingleLinkedList<Rc<String>>,
        arena: &Bump,
        ui: &mut Ui,
    ) -> bool {
        let mut modified = false;

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
                let key_path = key_path.snoc(key.clone());

                ui.push_id(key, |ui| match value {
                    Constants::Object { map } => {
                        modified |= Self::show_object(
                            key.clone(),
                            map,
                            output,
                            constants
                                .entry(key.clone())
                                .or_insert(Constants::Object {
                                    map: BTreeMap::new(),
                                })
                                .make_object_mut(),
                            &key_path,
                            arena,
                            ui,
                        );
                    }

                    Constants::Driver { default } => {
                        modified |= Self::show_value(
                            key.clone(),
                            &key_path,
                            constants.get_mut(key),
                            default,
                            output,
                            ui,
                            arena,
                        );
                    }

                    _ => {}
                });
            }
        });

        modified
    }

    fn show_value(
        name: Rc<String>,
        key_path: &SingleLinkedList<Rc<String>>,
        constant: Option<&mut Constants>,
        default: &Constants,
        output: &EventStream<GlobalEvents>,
        ui: &mut Ui,
        arena: &Bump,
    ) -> bool {
        ui.horizontal(|ui| match constant {
            Some(c) => {
                ui.label(bumpalo::format!(in &arena, "{} = ", name).as_str());
                let ret = crate::tabs::constants::ConstantsTab::modify_value(arena, c, ui);

                if ui.button("reset").clicked() {
                    output.add_event(GlobalEvents::RemoveOptionDriver(Rc::new(key_path.to_vec())));
                }

                ret
            }
            None => {
                if ui.button("change value").clicked() {
                    output.add_event(GlobalEvents::AddOptionDriver(
                        Rc::new(key_path.to_vec()),
                        default.clone(),
                    ));
                }

                true
            }
        })
        .inner
    }
}
