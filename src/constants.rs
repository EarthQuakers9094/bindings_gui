use std::{collections::BTreeMap, fmt::Display, rc::Rc};

use bumpalo::Bump;
use egui::{Id, Ui};
use serde::{Deserialize, Serialize};

use crate::search_selector::{search_selector, SelectorCache};

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(untagged)]
pub enum Constants {
    Driver {
        default: Box<Constants>,
    },

    Object {
        map: BTreeMap<Rc<String>, Constants>,
    },
    Float(f64),
    Int(i64),
    String(String),

    #[default]
    None,
}

impl Display for Constants {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constants::Object { map } => {
                write!(f, "{{")?;

                for (k, v) in map {
                    write!(f, "{k} = {v}; ")?;
                }

                write!(f, "}}")?;

                Ok(())
            }
            Constants::Float(a) => write!(f, "{a}"),
            Constants::Int(a) => write!(f, "{a}"),
            Constants::String(a) => write!(f, "\"{a}\""),
            Constants::Driver { default } => write!(f, "default: {default}"),
            Constants::None => write!(f, "null"),
        }
    }
}

impl Constants {
    pub fn default_for_type(t: ConstantsType, d: ConstantsType) -> Self {
        match t {
            ConstantsType::Object => Constants::Object {
                map: Default::default(),
            },
            ConstantsType::Float => Constants::Float(Default::default()),
            ConstantsType::Int => Constants::Int(Default::default()),
            ConstantsType::String => Constants::String(Default::default()),
            ConstantsType::Driver => {
                if d == ConstantsType::Driver {
                    panic!("can't have Driver<Driver>")
                } else {
                    Constants::Driver {
                        default: Box::new(Self::default_for_type(d, d)),
                    }
                }
            }
            ConstantsType::Null => Constants::None,
        }
    }

    pub fn get_object_mut(&mut self) -> &mut BTreeMap<Rc<String>, Constants> {
        match self {
            Constants::Object { map } => map,
            _ => panic!("invalid arguments"),
        }
    }

    pub fn add_option(&mut self, key: OptionLocation, value: Constants) -> bool {
        let mut cloc = self;

        for l in key.iter() {
            match cloc {
                Constants::Object { map } => {
                    cloc = map.entry(l.clone()).or_insert(Constants::None);
                }
                Constants::None => {
                    let mut map = BTreeMap::new();

                    map.insert(l.clone(), Constants::None);

                    *cloc = Constants::Object { map };

                    cloc = cloc.get_object_mut().get_mut(l).unwrap();
                }
                _ => return true,
            }
        }

        if *cloc != Constants::None {
            return true;
        }

        *cloc = value;

        false
    }

    pub fn remove_key(&mut self, key: &[Rc<String>]) {
        match key.len() {
            0 => panic!("invalid args"),
            1 => {
                self.get_object_mut().remove(&key[0]);
            }
            _ => {
                self.get_object_mut()
                    .get_mut(&key[0])
                    .unwrap()
                    .remove_key(&key[1..]);
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConstantsType {
    Object,
    Float,
    Int,
    String,

    Driver,

    #[default]
    Null,
}

impl Display for ConstantsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ConstantsType {
    fn name(&self) -> &'static str {
        match self {
            ConstantsType::Object => "Object",
            ConstantsType::Float => "Float",
            ConstantsType::Int => "Int",
            ConstantsType::String => "String",
            ConstantsType::Null => "null",
            ConstantsType::Driver => "Driver",
        }
    }

    pub fn valid_types(arena: &Bump, driver: bool) -> &mut dyn Iterator<Item = Self> {
        let non_driver = [
            ConstantsType::Object,
            ConstantsType::Float,
            ConstantsType::Int,
            ConstantsType::String,
            ConstantsType::Null,
        ];
        if driver {
            arena.alloc(non_driver.into_iter())
        } else {
            arena.alloc(
                non_driver
                    .into_iter()
                    .chain([ConstantsType::Driver].into_iter()),
            )
        }
    }

    pub fn selector(
        &mut self,
        filter: &mut String,
        cache: &mut SelectorCache<ConstantsType>,
        ui: &mut Ui,
        driver: bool,
        arena: &Bump,
        id: Id,
    ) {
        search_selector(
            id,
            filter,
            self,
            Self::valid_types(arena, driver)
                .into_iter()
                .map(|a| (Rc::new(a.to_string()), a)),
            cache,
            100.0,
            ui,
        );
    }
}

pub type OptionLocation = Rc<Vec<Rc<String>>>;
