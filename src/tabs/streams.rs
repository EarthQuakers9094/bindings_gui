use std::{collections::BTreeMap, rc::Rc};

use egui::{Grid, ScrollArea};

use crate::{
    global_state::{GlobalEvents, State},
    search_selector::{search_selector, SelectorCache},
    Component,
};

#[derive(Debug, Default)]
pub struct EditingStates {
    controller_filter: String,
    controller: u8,
    controller_cache: SelectorCache<u8>,

    axis_filter: String,
    axis: u8,
    axis_cache: SelectorCache<u8>,
}

#[derive(Debug, Default)]
pub struct StreamsTab {
    pub edit_state: BTreeMap<Rc<String>, EditingStates>,
}

impl Component for StreamsTab {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
        arena: &bumpalo::Bump,
    ) {
        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("streams tab grid").show(ui, |ui| {
                for ele in env.streams.iter() {
                    ui.label(ele.as_str());

                    match env.stream_to_axis.get(ele) {
                        Some((controller, axis)) => {
                            let axis =
                                env.controllers[*controller as usize].axis_name(*axis, arena);
                            let controller = env.controller_name(*controller);

                            ui.label(
                                bumpalo::format!(in &arena, "{} on {}", controller, axis).as_str(),
                            )
                        }
                        None => ui.label("Not Bound"),
                    };

                    let edit_state = self.edit_state.entry(ele.clone()).or_default();

                    ui.horizontal(|ui| {
                        search_selector(
                            ui.make_persistent_id(("streams controller selector", ele)),
                            &mut edit_state.controller_filter,
                            &mut edit_state.controller,
                            env.controllers.iter().enumerate().flat_map(|(id, c)| {
                                if c.bound() {
                                    Some((env.controller_name(id as u8), id as u8))
                                } else {
                                    None
                                }
                            }),
                            &mut edit_state.controller_cache,
                            100.0,
                            ui,
                        );

                        let controller =
                            env.controllers.get(edit_state.controller as usize).unwrap();

                        search_selector(
                            ui.make_persistent_id(("streams axis selector", ele)),
                            &mut edit_state.axis_filter,
                            &mut edit_state.axis,
                            controller.enumerate_axises().map(|axis| {
                                (Rc::new(controller.axis_name(axis, arena).to_string()), axis)
                            }),
                            &mut edit_state.axis_cache,
                            100.0,
                            ui,
                        );

                        if ui.button("change").clicked() {
                            output.add_event(GlobalEvents::SetStream(
                                ele.clone(),
                                edit_state.controller,
                                edit_state.axis,
                            ));
                        }
                    });

                    ui.end_row();
                }
            });
        });
    }
}
