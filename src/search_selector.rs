use std::rc::Rc;

use egui::{popup_below_widget, Id, TextEdit, Ui};

#[derive(Debug, Default)]
pub struct SingleCache<K, V> {
    last_key: Option<K>,
    value: V,
    read: bool,
}

impl<K, V> SingleCache<K, V> {
    pub fn get<F>(&mut self, key: &K, f: F) -> &V
    where
        F: FnOnce() -> V,
        K: PartialEq + Clone,
    {
        self.read = true;
        if self.last_key.as_ref() != Some(key) {
            self.last_key = Some(key.clone());
            self.value = f();
        }

        &self.value
    }

    pub fn update(&mut self) {
        if !self.read {
            self.last_key = None;
        }
        self.read = false;
    }
}

pub type SelectorCache<A> = SingleCache<String, Vec<(Rc<String>, A)>>;

pub(crate) fn search_selector<A>(
    id: Id,
    text: &mut String,
    selection: &mut A,
    options: impl Iterator<Item = (Rc<String>, A)>,
    cache: &mut SelectorCache<A>,
    width: f32,
    ui: &mut Ui,
) -> bool
where
    A: Clone,
{
    let edit = ui.add(TextEdit::singleline(text).desired_width(width));

    let mut changed = false;

    if edit.gained_focus() {
        ui.memory_mut(|mem| mem.open_popup(id));
    }

    popup_below_widget(
        ui,
        id,
        &edit,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            let vals = cache.get(text, || {
                options
                    .filter(|(name, _value)| name.contains(text.as_str()))
                    .take(10)
                    .collect::<Vec<_>>()
            });

            if vals.len() == 1 {
                *selection = vals[0].1.clone();
                changed = true;
            }

            for (name, value) in vals {
                if ui.button(name.as_str()).clicked() {
                    changed = true;
                    *selection = value.clone();
                    ui.memory_mut(|mem| mem.close_popup());
                    text.clear();

                    text.push_str(name.as_str());
                }
            }
        },
    );

    cache.update();

    changed
}
