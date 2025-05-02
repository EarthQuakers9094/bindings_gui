use anyhow::Result;
use component::Compenent;
use egui::{Align2, Direction, Ui};
use egui_dock::{DockArea, DockState, Style, TabViewer};
use egui_toast::{Toast, Toasts};
use global_state::{GlobalEvents, Views};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;

mod component;
mod from_binding;
mod from_commands;
mod global_state;
mod managetab;

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
enum Button {
    Button(u8),
    Pov(i16),
}

impl Display for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Button::Button(b) => b.fmt(f),
            Button::Pov(pov) => match pov {
                0 => write!(f, "up"),
                45 => write!(f, "up left"),
                90 => write!(f, "right"),
                135 => write!(f, "down right"),
                180 => write!(f, "down"),
                225 => write!(f, "down left"),
                270 => write!(f, "left"),
                315 => write!(f, "up left"),
                -1 => write!(f, "no pov"),
                _ => write!(f, "ERROR"),
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
enum RunWhen {
    OnTrue,
    OnFalse,
    WhileTrue,
    WhileFalse,
}

impl RunWhen {
    fn get_str(self) -> &'static str {
        match self {
            RunWhen::OnTrue => "on true",
            RunWhen::OnFalse => "on false",
            RunWhen::WhileTrue => "while true",
            RunWhen::WhileFalse => "while false",
        }
    }
}

impl Display for RunWhen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_str())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
struct Binding {
    controller: u8,
    button: Button,
    when: RunWhen,
}

impl Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.controller, self.button, self.when)
    }
}

#[derive(Debug)]
enum App {
    Initial { error: Option<String> },

    Running { views: Views, tree: DockState<Tab> },
}

impl Default for App {
    fn default() -> Self {
        Self::Initial { error: None }
    }
}

impl App {
    fn from_views(view: Views) -> Self {
        Self::Running {
            views: view,
            tree: DockState::new(vec![
                Tab {
                    tab: Box::new(from_commands::FromCommands {
                        ..Default::default()
                    }),
                    name: "commands to bindings",
                },
                Tab {
                    tab: Box::new(from_binding::FromBindings {
                        ..Default::default()
                    }),
                    name: "bindings to commands",
                },
                Tab {
                    tab: Box::new(managetab::ManageTab {
                        ..Default::default()
                    }),
                    name: "manage commands",
                },
            ]),
        }
    }

    fn initial(&self) -> bool {
        match self {
            Self::Initial { .. } => true,
            _ => false,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);

        if self.initial() {
            egui::CentralPanel::default().show(ctx, |ui| match self {
                Self::Initial { error } => {
                    if ui.button("Open Project Directory").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            match Views::from_directory(path) {
                                Ok(s) => {
                                    *self = Self::from_views(s);
                                    return;
                                }
                                Err(err) => *error = Some(err.to_string()),
                            }
                        }
                    }

                    if let Some(err) = error {
                        ui.label(err.as_str());
                    }
                }
                App::Running { .. } => panic!("impossible"),
            });
        }

        match self {
            App::Initial { .. } => {}
            App::Running { views, tree } => {
                let mut toasts = Toasts::new()
                    .anchor(Align2::LEFT_BOTTOM, (-10.0, -10.0))
                    .direction(Direction::BottomUp);

                DockArea::new(tree)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show_close_buttons(false)
                    .show_add_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .show_leaf_close_all_buttons(false)
                    .show(
                        ctx,
                        &mut Tabs {
                            view: views,
                            toasts: &mut toasts,
                        },
                    );

                toasts.show(ctx);
            }
        }
    }
}

#[derive(Debug)]
struct BindingsMap {
    command_to_bindings: BTreeMap<String, Vec<Binding>>,
    binding_to_command: BTreeMap<(u8, Button), Vec<(String, RunWhen)>>,
}

impl Default for BindingsMap {
    fn default() -> Self {
        Self {
            command_to_bindings: Default::default(),
            binding_to_command: Default::default(),
        }
    }
}

impl BindingsMap {
    fn add_binding(&mut self, command: String, binding: Binding) -> bool {
        if self
            .command_to_bindings
            .get(&command)
            .unwrap_or(&Vec::new())
            .contains(&binding)
        {
            false
        } else {
            self.command_to_bindings
                .entry(command.clone())
                .or_insert(Vec::new())
                .push(binding);

            self.binding_to_command
                .entry((binding.controller, binding.button))
                .or_insert(Vec::new())
                .push((command.clone(), binding.when));

            true
        }
    }

    fn remove_binding(&mut self, command: &String, binding: Binding) {
        self.command_to_bindings
            .get_mut(command)
            .unwrap()
            .retain(|b| *b != binding);
        self.binding_to_command
            .get_mut(&(binding.controller, binding.button))
            .unwrap()
            .retain(|(c, when): &(String, RunWhen)| !(command == c && *when == binding.when));
    }

    fn is_used(&self, command: &String) -> bool {
        self.command_to_bindings
            .get(command)
            .map_or(false, |l| !l.is_empty())
    }

    fn has_binding(&self, command: &String, binding: Binding) -> bool {
        self.command_to_bindings
            .get(command)
            .map_or(false, |bindings| bindings.contains(&binding))
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Bindings<'a> {
    url: Cow<'a, Option<String>>,

    commands: Cow<'a, BTreeSet<String>>,
    command_to_bindings: Cow<'a, BTreeMap<String, Vec<Binding>>>,
}

#[derive(Debug)]
enum ProgramError {
    NotDirectory(PathBuf),
    ExistingDirectoryAt(PathBuf),
}

impl Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramError::NotDirectory(path_buf) => {
                write!(f, "{} is not a directory", path_buf.display())
            }
            ProgramError::ExistingDirectoryAt(path_buf) => {
                write!(f, "a directory exists at {}, aborting", path_buf.display())
            }
        }
    }
}

impl Error for ProgramError {}

struct Tabs<'a> {
    view: &'a mut Views,
    toasts: &'a mut Toasts,
}

impl<'a> Tabs<'a> {
    fn add_error(&mut self, error: String) {
        self.toasts.add(Toast {
            kind: egui_toast::ToastKind::Error,
            text: error.into(),
            ..Default::default()
        });
    }
}

#[derive(Debug)]
struct Tab {
    tab: Box<dyn Compenent<OutputEvents = GlobalEvents, Environment = Views>>,
    name: &'static str,
}

impl<'a> TabViewer for Tabs<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.name.into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match self.view.display_tab(ui, tab, &mut self.toasts) {
            Ok(_) => {}
            Err(err) => {
                self.add_error(err.to_string());
            }
        };
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

fn main() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((400.0, 300.0)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        "Bindings",
        native_options,
        Box::new(|_| Ok(Box::<App>::default())),
    )
}
