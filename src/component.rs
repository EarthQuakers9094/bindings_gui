use egui::Ui;
use std::cell::RefCell;

#[derive(Debug, Default)]
pub(crate) struct EventStream<E> {
    events: RefCell<Vec<E>>,
}

impl<E> EventStream<E> {
    pub(crate) fn add_event(&self, e: E) {
        self.events.borrow_mut().push(e);
    }

    pub(crate) fn new() -> Self {
        EventStream {
            events: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        self.events
            .get_mut()
            .drain(0..)
    }
}

pub(crate) trait Compenent: std::fmt::Debug {
    type OutputEvents;
    type Environment;

    fn render(
        &mut self,
        ui: &mut Ui,
        env: &Self::Environment,
        output: &EventStream<Self::OutputEvents>,
    );
}
