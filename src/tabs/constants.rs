use std::{
    collections::{BTreeMap, HashMap},
    mem,
    ops::DerefMut,
    rc::Rc,
};

use bumpalo::Bump;
use egui::{collapsing_header::CollapsingState, CollapsingHeader, ComboBox, ScrollArea, Ui};
use egui_hooks::UseHookExt;

use crate::{
    component::EventStream,
    constants::{Constants, ConstantsType, OptionLocation},
    global_state::{GlobalEvents, State},
    number_input::number_input,
    search_selector::SelectorCache,
    Component,
};

#[derive(Debug, Default, Clone)]
pub struct EditingStates {
    t: ConstantsType,
    name: String,

    type_filters: Vec<String>,
    type_caches: Vec<SelectorCache<ConstantsType>>,
}

#[derive(Debug, Default, Clone)]
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
                        ui.push_id(key, |ui| match value {
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
                        });
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
        super::TabType::Constants
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
                &mut state.type_filters,
                &mut state.type_caches,
                ui,
                false,
                arena,
                ui.make_persistent_id(("adding id", &key)),
            );

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
                    Constants::default_for_type(&state.t),
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
            let ret = Self::modify_value(arena, constant, ui);

            if ui.button("X").clicked() {
                let k = Rc::make_mut(&mut key_path);
                k.push(name);
                output.add_event(GlobalEvents::RemoveOption(dbg!(key_path)));
            }

            ret
        })
        .inner
    }

    pub fn modify_value(arena: &Bump, constant: &mut Constants, ui: &mut Ui) -> bool {
        match constant {
            Constants::Object { .. } => panic!("invalid argument"),
            Constants::Float(f) => {
                let mut s = ui.use_state(|| f.to_string(), ()).into_var();

                number_input(s.deref_mut(), f, arena, ui)
            }
            Constants::Int(i) => {
                let mut s = ui.use_state(|| i.to_string(), ()).into_var();

                number_input(s.deref_mut(), i, arena, ui)
            }
            Constants::String(s) => ui.text_edit_singleline(s).lost_focus(),
            Constants::Driver { default } => {
                ui.label("default");
                Self::modify_value(arena, default.as_mut(), ui)
            }
            Constants::None => {
                ui.label("null");
                false
            }
            Constants::List(items, constants_type) => {
                let mut update = false;

                CollapsingHeader::new("").show(ui, |ui| {
                    let mut id = 0;

                    items.retain_mut(|i| {
                        ui.horizontal(|ui| {
                            update |= ui.push_id(id, |ui| Self::modify_value(arena, i, ui)).inner;
                            id += 1;

                            !ui.button("X").clicked()
                        })
                        .inner
                    });

                    if ui.button("add").clicked() {
                        items.push(Constants::default_for_type(constants_type));
                    }
                });

                update
            }
            Constants::Bool(value) => {
                let mut updated = false;

                ComboBox::from_label("")
                    .selected_text(value.to_string())
                    .show_ui(ui, |ui| {
                        for i in [true, false] {
                            updated |= ui.selectable_value(value, i, i.to_string()).changed();
                        }
                    });

                updated
            }
        }
    }
}
