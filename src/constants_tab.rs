use std::{
    collections::{BTreeMap, HashMap},
    mem,
    rc::Rc,
};

use bumpalo::Bump;
use egui::{collapsing_header::CollapsingState, DragValue, ScrollArea, Ui};

use crate::{
    component::EventStream,
    constants::{Constants, ConstantsType, OptionLocation},
    global_state::{GlobalEvents, State},
    search_selector::SelectorCache,
    Component,
};

#[derive(Debug, Default)]
pub struct EditingStates {
    t: ConstantsType,
    name: String,
    type_filter: String,
    type_filter_cache: SelectorCache<ConstantsType>,

    driver_type: ConstantsType,
    driver_type_filter: String,
    driver_type_filter_cache: SelectorCache<ConstantsType>,
}

#[derive(Debug, Default)]
pub struct ConstantsTab {
    pub add: HashMap<OptionLocation, EditingStates>,
}

impl Component for ConstantsTab {
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
            let constants = &mut env.constants;

            self.add_dialog(Rc::new(Vec::new()), output, arena, ui);

            match constants {
                Constants::Object { map } => {
                    for (key, value) in map.iter_mut() {
                        match value {
                            Constants::Object { map } => {
                                modified |= self.show_object(
                                    key.clone(),
                                    map,
                                    Rc::new(Vec::new()),
                                    output,
                                    arena,
                                    ui,
                                );
                            }
                            _ => {
                                modified |= Self::show_value(
                                    key.clone(),
                                    Rc::new(Vec::new()),
                                    value,
                                    ui,
                                    output,
                                    arena,
                                )
                            }
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

impl ConstantsTab {
    fn add_dialog(
        &mut self,
        mut key: Rc<Vec<Rc<String>>>,
        output: &EventStream<GlobalEvents>,
        arena: &Bump,
        ui: &mut Ui,
    ) {
        let state = match self.add.get_mut(&key) {
            Some(a) => a,
            None => {
                self.add.insert(key.clone(), Default::default());
                self.add.get_mut(&key).unwrap()
            }
        };

        ui.horizontal(|ui| {
            ui.label("name:");
            ui.text_edit_singleline(&mut state.name);

            ui.label("type:");
            state.t.selector(
                &mut state.type_filter,
                &mut state.type_filter_cache,
                ui,
                false,
                arena,
                ui.make_persistent_id(("adding id", &key)),
            );

            if state.t == ConstantsType::Driver {
                state.driver_type.selector(
                    &mut state.driver_type_filter,
                    &mut state.driver_type_filter_cache,
                    ui,
                    true,
                    arena,
                    ui.make_persistent_id(("adding id driver", &key)),
                );
            }

            if ui.button("add").clicked() {
                if state.name.is_empty() {
                    output.add_event(GlobalEvents::DisplayError(
                        "no name provided for event".to_string(),
                    ));
                    return;
                }

                let k = Rc::make_mut(&mut key);
                k.push(Rc::new(mem::take(&mut state.name)));

                output.add_event(GlobalEvents::AddOption(
                    key,
                    Constants::default_for_type(state.t, state.driver_type),
                ));
            }
        });
    }

    fn show_object(
        &mut self,
        name: Rc<String>,
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
            if constants.is_empty() && ui.button("X").clicked() {
                output.add_event(GlobalEvents::RemoveOption(key_path.clone()));
            }
        })
        .body(|ui| {
            self.add_dialog(key_path.clone(), output, arena, ui);

            for (key, value) in constants {
                match value {
                    Constants::Object { map } => {
                        modified |=
                            self.show_object(key.clone(), map, key_path.clone(), output, arena, ui);
                    }
                    _ => {
                        modified |= Self::show_value(
                            key.clone(),
                            key_path.clone(),
                            value,
                            ui,
                            output,
                            arena,
                        )
                    }
                }
            }
        });

        modified
    }

    fn show_value(
        name: Rc<String>,
        mut key_path: OptionLocation,
        constant: &mut Constants,
        ui: &mut Ui,
        output: &EventStream<GlobalEvents>,
        arena: &Bump,
    ) -> bool {
        ui.horizontal(|ui| {
            ui.label(bumpalo::format!(in &arena, "{} = ", name).as_str());
            let ret = Self::modify_value(constant, ui);

            if ui.button("X").clicked() {
                let k = Rc::make_mut(&mut key_path);
                k.push(name);
                output.add_event(GlobalEvents::RemoveOption(key_path));
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
