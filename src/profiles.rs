use std::{mem, rc::Rc};

use crate::{
    global_state::{GlobalEvents, State},
    search_selector::{search_selector, SingleCache},
    Component,
};

#[derive(Debug)]
pub(crate) struct ProfilesTab {
    pub name: String,
    pub filter: String,
    pub profile_selection: Rc<String>,
    pub filter_cache: SingleCache<String, Vec<(Rc<String>, Rc<String>)>>
}

impl Default for ProfilesTab {
    fn default() -> Self {
        Self { name: Default::default(), filter: Default::default(), profile_selection: Default::default(), filter_cache: Default::default() }
    }
}

impl Component for ProfilesTab {
    type OutputEvents = GlobalEvents;

    type Environment = State;

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
        _arena: &bumpalo::Bump,
    ) {
        ui.horizontal(|ui| {
            ui.label("new profile: ");
            ui.text_edit_singleline(&mut self.name);
            if ui.button("add").clicked() {
                output.add_event(GlobalEvents::AddProfile(mem::take(&mut self.name)));
            }
        });

        ui.horizontal(|ui| {
            ui.label("active_profile: ");

            ui.label(env.profile.as_str());

            self.profile_selection = env.profile.clone();

            if search_selector(
                            ui.make_persistent_id("profiles selector"),
                            &mut self.filter,
                            &mut self.profile_selection,
                            env.profiles.iter().map(|s| (s.clone(), s.clone())),
                            &mut self.filter_cache,
                            300.0,
                            ui,
                        ) {
                output.add_event(GlobalEvents::SetProfile(self.profile_selection.clone()));
            };
        });
    }
}
