use egui::TextEdit;

use crate::Component;

#[derive(Debug, Clone)]
pub struct PasswordLock<A> {
    component: A,
    locked: bool,
    password_typed: String,
}

impl<A> PasswordLock<A> {
    pub fn new(a: A) -> Self {
        Self {
            component: a,
            locked: true,
            password_typed: "".to_string(),
        }
    }
}

impl<A> Component for PasswordLock<A>
where
    A: Component,
{
    type OutputEvents = A::OutputEvents;

    type Environment = A::Environment;

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut Self::Environment,
        output: &crate::component::EventStream<Self::OutputEvents>,
        arena: &bumpalo::Bump,
    ) {
        if self.locked {
            ui.horizontal(|ui| {
                ui.label("password: ");

                if ui
                    .add(
                        TextEdit::singleline(&mut self.password_typed)
                            .password(true)
                            .desired_width(100.0),
                    )
                    .lost_focus()
                    && self.password_typed == "theyWillNeverKnow!"
                {
                    self.password_typed.clear();
                    self.locked = false;
                }
            });
        } else {
            if ui.button("lock").clicked() {
                self.locked = true;
            }
            self.component.render(ui, env, output, arena);
        }
    }

    fn tab_type(&self) -> super::TabType {
        self.component.tab_type()
    }
}
