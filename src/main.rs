#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::Result;
use bumpalo::Bump;
use component::Component;
use egui::{Align2, Direction, ScrollArea, Ui};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabViewer};
use egui_toast::{Toast, Toasts};
use global_state::{GlobalEvents, State};
use once_cell::sync::Lazy;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;

mod bindings;
mod component;
mod constants;
mod global_state;
mod search_selector;
mod single_linked_list;
mod tabs;

use tabs::*;

// for when external event loop support is added
// mod sync_thread;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum App {
    Initial {
        error: Option<String>,
    },

    Running {
        views: State,
        tree: DockState<Tab>,
        arena: Bump,
        used_tabs: BTreeSet<TabType>,
    },
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
            tree: DockState::new(vec![Tab {
                tab: None,
                name: "new tab",
            }]),
            arena: Bump::new(),
            used_tabs: BTreeSet::new(),
        }
    }

    fn initial(&self) -> bool {
        matches!(self, Self::Initial { .. })
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        match self {
            App::Initial { .. } => {}
            App::Running { views, .. } => {
                if let Some(p) = &mut views.sync_process {
                    p.kill().unwrap()
                }
            }
        }
    }

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
            App::Running {
                views,
                tree,
                arena,
                used_tabs,
            } => {
                let mut toasts = Toasts::new()
                    .anchor(Align2::LEFT_BOTTOM, (-10.0, -10.0))
                    .direction(Direction::BottomUp);

                let mut added_nodes = Vec::new();

                DockArea::new(tree)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show_add_buttons(true)
                    .show_leaf_collapse_buttons(false)
                    .show(
                        ctx,
                        &mut Tabs {
                            view: views,
                            toasts: &mut toasts,
                            arena,
                            added_nodes: &mut added_nodes,
                            used_tabs,
                        },
                    );

                for i in added_nodes {
                    tree.set_focused_node_and_surface(i);
                    tree.push_to_focused_leaf(Tab {
                        tab: None,
                        name: "new tab",
                    });
                }

                if tree.main_surface().is_empty() {
                    println!("adding new tab");
                    tree.push_to_first_leaf(Tab {
                        tab: None,
                        name: "new tab",
                    });
                }

                if let Some(child) = &mut views.sync_process {
                    match child.try_wait() {
                        Ok(exit) => {
                            if let Some(status) = exit {
                                if !status.success() {
                                    toasts.add(Toast {
                                        kind: egui_toast::ToastKind::Error,
                                        text: "failed to sync".into(),
                                        ..Default::default()
                                    });
                                }

                                println!("exited");

                                views.sync_process = None;
                            }
                        }
                        Err(err) => {
                            toasts.add(Toast { kind: egui_toast::ToastKind::Error, text: bumpalo::format!(in &arena, "failed to wait on sync process {}", err).as_str().into(), ..Default::default() });
                        }
                    }
                }

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
    arena: &'a mut Bump,
    added_nodes: &'a mut Vec<(SurfaceIndex, NodeIndex)>,
    used_tabs: &'a mut BTreeSet<tabs::TabType>,
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
    tab: Option<Box<dyn Component<OutputEvents = GlobalEvents, Environment = State>>>,
    name: &'static str,
}

impl TabViewer for Tabs<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.name.into()
    }

    fn on_add(&mut self, _surface: egui_dock::SurfaceIndex, _node: egui_dock::NodeIndex) {
        self.added_nodes.push((_surface, _node));
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        if let Some(t) = &tab.tab {
            self.used_tabs.remove(&t.tab_type());
        }

        true
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match &mut tab.tab {
            Some(t) => {
                match self.view.display_tab(ui, t, self.toasts, self.arena) {
                    Ok(_) => {}
                    Err(err) => {
                        self.add_error(err.to_string());
                    }
                };
            }
            None => {
                ScrollArea::vertical().show(ui, |ui| {
                    let mut new_tab: Option<TabType> = None;

                    for i in Lazy::force(&ALL_TABS).difference(self.used_tabs) {
                        if ui.button(i.name()).clicked() {
                            new_tab = Some(*i);

                            let b = i.build();

                            tab.tab = Some(b);
                            tab.name = i.name();
                        }
                    }

                    if let Some(t) = new_tab {
                        self.used_tabs.insert(t);
                    }
                });
            }
        }

        self.arena.reset();
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

    // FOR WHEN EXTERNAL EVENTLOOP SUPPORT IS ADDED
    // let eventloop = EventLoop::<UserEvent>::with_user_event().build().unwrap();

    // eventloop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    // let mut winit_app = eframe::create_native(
    //     "Bindings",
    //     native_options,
    //     Box::new(|_| Ok(Box::<App>::default())),
    //     &eventloop,
    // );

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
