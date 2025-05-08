use egui::{popup_below_widget, Id, TextEdit, Ui};

#[derive(Debug, Default)]
pub struct SingleCash<K, V> {
    last_key: Option<K>,
    value: V,
    read: bool,
}

impl<K, V> SingleCash<K, V> {
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
    }
}

pub(crate) fn search_selector<'a, A>(
    id: Id,
    text: &mut String,
    selection: &mut A,
    options: impl Iterator<Item = (String, A)>,
    cache: &mut SingleCash<String, Vec<(String, A)>>,
    width: f32,
    ui: &mut Ui,
) where A: Clone {
    let edit = ui.add(TextEdit::singleline(text).desired_width(width));

    if edit.gained_focus() {
        ui.memory_mut(|mem| mem.open_popup(id));
    }

    popup_below_widget(
        ui,
        id,
        &edit,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            let vals = cache.get(&text, || {
                options
                    .filter(|(name, _value)| name.contains(text.as_str()))
                    .take(10)
                    .map(|(a,b)| (a.clone(),b))
                    .collect::<Vec<_>>()
            });
            
            if vals.len() == 1 {
                *selection = vals[0].1.clone();
            }

            for (name, value) in vals {
                if ui.button(name).clicked() {
                    *selection = value.clone();
                    ui.memory_mut(|mem| mem.close_popup());
                    *text = name.clone();
                }
            }
        },
    );
}
