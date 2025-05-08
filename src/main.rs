use anyhow::Result;
use component::Component;
use egui::{Align2, Direction, Ui};
use egui_dock::{DockArea, DockState, Style, TabViewer};
use egui_toast::{Toast, Toasts};
use global_state::{GlobalEvents, State};
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;

mod bindings;
mod component;
mod from_binding;
mod from_commands;
mod global_state;
mod manage_controllers;
mod manage_commands;
mod search_selector;

#[derive(Debug)]
enum App {
    Initial { error: Option<String> },

    Running { views: State, tree: DockState<Tab> },
}

impl Default for App {
    fn default() -> Self {
        Self::Initial { error: None }
    }
}

impl App {
    fn from_views(view: State) -> Self {
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
                    tab: Box::new(manage_commands::ManageTab {
                        ..Default::default()
                    }),
                    name: "manage commands",
                },
                Tab {
                    tab: Box::new(manage_controllers::ManageControllers {
                        ..Default::default()
                    }),
                    name: "manage controllers",
                },
            ]),
        }
    }

    fn initial(&self) -> bool {
        matches!(self, Self::Initial { .. })
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
                            match State::from_directory(path) {
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
    view: &'a mut State,
    toasts: &'a mut Toasts,
}

impl Tabs<'_> {
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
    tab: Box<dyn Component<OutputEvents = GlobalEvents, Environment = State>>,
    name: &'static str,
}

impl TabViewer for Tabs<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.name.into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match self.view.display_tab(ui, tab, self.toasts) {
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

// fn main() {
//     let command_prefix = "Hello World".to_string();

//     let mut command = 0;

//     let mut command_to_bindings = BTreeMap::new();
//     let mut commands = BTreeSet::new();

//     for controller in 0..5 {
//         for binding in 1..33 {
//             for when in RunWhen::enumerate() {
//                 command_to_bindings.insert(
//                     format!("{command_prefix}{command}"),
//                     vec![Binding {
//                         controller: controller,
//                         button: Button {
//                             button: binding,
//                             location: bindings::ButtonLocation::Button,
//                         },
//                         during: when,
//                     }],
//                 );

//                 commands.insert(format!("{command_prefix}{command}"));

//                 command += 1;
//             }
//         }
//         for pov in [-1, 0, 45, 90, 135, 180, 225, 270] {
//             for when in RunWhen::enumerate() {
//                 command_to_bindings.insert(
//                     format!("{command_prefix}{command}"),
//                     vec![Binding {
//                         controller: controller,
//                         button: Button {
//                             button: pov,
//                             location: bindings::ButtonLocation::Pov,
//                         },
//                         during: when,
//                     }],
//                 );

//                 commands.insert(format!("{command_prefix}{command}"));

//                 command += 1;
//             }
//         }
//     }

//     let savedata = SaveData {
//         url: Cow::Owned(None),
//         commands: Cow::Owned(commands),
//         command_to_bindings: Cow::Owned(command_to_bindings),
//     };

//     let mut file = File::create("worse_case_senario.json").unwrap();

//     file.write_all(serde_json::to_string(&savedata).unwrap().as_bytes())
//         .unwrap();
// }
