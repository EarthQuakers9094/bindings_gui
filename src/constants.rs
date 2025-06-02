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
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    List(Vec<Constants>, ConstantsType),
    Meters {
        distance: f64,
    },
    Degrees {
        degrees: f64,
    },

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
            Constants::List(items, _type) => {
                write!(f, "{{")?;

                for item in items {
                    write!(f, "{item}, ")?;
                }

                write!(f, "}}")?;

                Ok(())
            }
            Constants::Bool(b) => {
                write!(f, "{b}")?;

                Ok(())
            }
            Constants::Meters { distance } => write!(f, "{distance} m"),
            Constants::Degrees { degrees } => {
                write!(f, "{degrees} ")
            }
        }
    }
}

impl Constants {
    pub fn default_for_type(t: &ConstantsType) -> Self {
        match t {
            ConstantsType::Object => Constants::Object {
                map: Default::default(),
            },
            ConstantsType::Float => Constants::Float(Default::default()),
            ConstantsType::Int => Constants::Int(Default::default()),
            ConstantsType::String => Constants::String(Default::default()),
            ConstantsType::Driver(d) => {
                if let ConstantsType::Driver(_) = d.as_ref() {
                    panic!("can't have Driver<Driver>")
                } else {
                    Constants::Driver {
                        default: Box::new(Self::default_for_type(d)),
                    }
                }
            }
            ConstantsType::Null => Constants::None,
            ConstantsType::List(t) => Constants::List(Vec::new(), t.as_ref().clone()),
            ConstantsType::Bool => Constants::Bool(false),
            ConstantsType::Distance => Constants::Meters { distance: 0.0 },
            ConstantsType::Angle => Constants::Degrees { degrees: 0.0 },
        }
    }

    pub fn get_object_mut(&mut self) -> &mut BTreeMap<Rc<String>, Constants> {
        match self {
            Constants::Object { map } => map,
            _ => panic!("invalid arguments"),
        }
    }

    pub fn make_object_mut(&mut self) -> &mut BTreeMap<Rc<String>, Constants> {
        match self {
            Constants::Object { map } => map,
            _ => {
                *self = Constants::Object {
                    map: BTreeMap::new(),
                };
                self.get_object_mut()
            }
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
        if *self == Constants::None {
            return;
        }

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

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ConstantsType {
    Object,
    Float,
    Int,
    String,
    Bool,
    Distance,
    Angle,

    Driver(Box<ConstantsType>),
    List(Box<ConstantsType>),

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
            ConstantsType::Driver(..) => "Driver",
            ConstantsType::List(..) => "List",
            ConstantsType::Bool => "Bool",
            ConstantsType::Distance => "Distance",
            ConstantsType::Angle => "Angle",
        }
    }

    pub fn valid_types(arena: &Bump, driver: bool) -> &mut dyn Iterator<Item = Self> {
        let non_driver = [
            ConstantsType::Float,
            ConstantsType::Int,
            ConstantsType::String,
            ConstantsType::Angle,
            ConstantsType::Distance,
            ConstantsType::List(Box::new(Self::Null)),
        ];
        if driver {
            arena.alloc(non_driver.into_iter())
        } else {
            arena.alloc(non_driver.into_iter().chain([
                ConstantsType::Driver(Box::new(Self::Null)),
                ConstantsType::Object,
            ]))
        }
    }

    fn selector_go(
        &mut self,
        filters: &mut Vec<String>,
        caches: &mut Vec<SelectorCache<ConstantsType>>,
        ui: &mut Ui,
        driver: bool,
        arena: &Bump,
        id: Id,
        loc: usize,
    ) {
        if filters.len() <= loc {
            filters.push(Default::default());
            caches.push(Default::default());
        }

        search_selector(
            (id, loc),
            &mut filters[loc],
            self,
            Self::valid_types(arena, driver).map(|a| (Rc::new(a.to_string()), a)),
            &mut caches[loc],
            100.0,
            ui,
        );

        match self {
            ConstantsType::Driver(constants_type) => {
                constants_type.selector_go(filters, caches, ui, true, arena, id, loc + 1);
            }
            ConstantsType::List(constants_type) => {
                constants_type.selector_go(filters, caches, ui, true, arena, id, loc + 1);
            }
            _ => {
                if filters.len() >= loc {
                    filters.truncate(loc + 1);
                }
            }
        }
    }

    pub fn selector(
        &mut self,
        filters: &mut Vec<String>,
        caches: &mut Vec<SelectorCache<ConstantsType>>,
        ui: &mut Ui,
        driver: bool,
        arena: &Bump,
        id: Id,
    ) {
        self.selector_go(filters, caches, ui, driver, arena, id, 0);
    }
}

pub type OptionLocation = Rc<Vec<Rc<String>>>;
