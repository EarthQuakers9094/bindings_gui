use egui::Ui;

#[derive(Debug, Default)]
pub(crate) struct EventStream<E> {
    events: Vec<E>,
}

impl<E> EventStream<E> {
    pub(crate) fn add_event(&mut self, e: E) {
        self.events.push(e);
    }

    pub(crate) fn new() -> Self {
        EventStream { events: Vec::new() }
    }

    pub(crate) fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        self.events.drain(0..)
    }
}

pub(crate) trait Compenent: std::fmt::Debug {
    type OutputEvents;
    type Environment;

    fn render(
        &mut self,
        ui: &mut Ui,
        env: &Self::Environment,
        output: &mut EventStream<Self::OutputEvents>,
    );
}
