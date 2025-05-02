use anyhow::Result;
use component::Compenent;
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
mod managetab;

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
                    tab: Box::new(managetab::ManageTab {
                        ..Default::default()
                    }),
                    name: "manage commands",
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
    tab: Box<dyn Compenent<OutputEvents = GlobalEvents, Environment = State>>,
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
