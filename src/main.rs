use anyhow::Result;
use egui::{ScrollArea, Ui};
use egui_dock::{DockArea, DockState, Style, TabViewer};
use from_commands::FromCommands;
use managetab::ManageTab;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::Display;
use std::fs::read_to_string;
use std::ops::Deref;
use std::path::PathBuf;

mod from_commands;
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
enum PageId {
    CommandsToBindings,
    BindingsToCommands,
    ManageCommands,
}

#[derive(Debug)]
enum App {
    Initial {
        error: Option<String>,
    },

    Running {
        views: Views,
        tree: DockState<PageId>,
    },
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
                PageId::CommandsToBindings,
                PageId::BindingsToCommands,
                PageId::ManageCommands,
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
                DockArea::new(tree)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show(ctx, views);
            }
        }
    }
}

impl TabViewer for Views {
    type Tab = PageId;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            PageId::CommandsToBindings => "commands to bindings".into(),
            PageId::BindingsToCommands => "bindings to commands".into(),
            PageId::ManageCommands => "manage commands".into(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let update = match tab {
            PageId::CommandsToBindings => from_commands::FromCommands::ui(ui, self),
            PageId::BindingsToCommands => false,
            PageId::ManageCommands => ManageTab::ui(ui, self),
        };
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

#[derive(Debug)]
struct Views {
    directory: PathBuf,

    url: Option<String>,

    commands: BTreeSet<String>,
    command_to_bindings: BTreeMap<String, Vec<Binding>>,

    binding_to_command: BTreeMap<Binding, Vec<String>>,

    manage_tab: managetab::ManageTab,
    from_commands: from_commands::FromCommands,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Bindings {
    url: Option<String>,

    commands: BTreeSet<String>,
    command_to_bindings: BTreeMap<String, Vec<Binding>>,
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

impl Views {
    fn from_bindings(bindings: Bindings, path: PathBuf) -> Self {
        let mut binding_to_command = BTreeMap::new();

        for (command, bindings) in bindings.command_to_bindings.iter() {
            for b in bindings {
                binding_to_command
                    .entry(*b)
                    .or_insert(Vec::new())
                    .push(command.clone());
            }
        }

        Self {
            directory: path,
            url: bindings.url,
            commands: bindings.commands,
            command_to_bindings: bindings.command_to_bindings,
            binding_to_command,
            manage_tab: ManageTab::default(),
            from_commands: FromCommands::default(),
        }
    }

    fn from_directory(mut path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            return Err(ProgramError::NotDirectory(path))?;
            // return format!("{} is not a directory", path.display())?;
        }

        let oldpath = path.clone();

        path.push("src");
        path.push("main");
        path.push("bindings.json");

        if path.is_dir() {
            // return is here just to convice the borrow checker that this path never
            // continues executing the function
            return Err(ProgramError::ExistingDirectoryAt(path))?;
        }

        if !path.exists() {
            return Ok(Self {
                directory: oldpath,
                url: None,
                commands: BTreeSet::new(),
                command_to_bindings: BTreeMap::new(),
                binding_to_command: BTreeMap::new(),
                manage_tab: ManageTab::default(),
                from_commands: FromCommands::default(),
            });
        }

        let file = read_to_string(path)?;

        let bindings: Bindings = serde_json::from_str(&file)?;

        Ok(Self::from_bindings(bindings, oldpath))
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
