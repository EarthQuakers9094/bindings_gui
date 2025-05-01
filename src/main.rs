use anyhow::{Context, Result};
use egui::{Align2, Ui};
use egui_dock::{DockArea, DockState, Style, TabViewer};
use egui_toast::{Toast, Toasts};
use from_commands::FromCommands;
use managetab::ManageTab;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::Display;
use std::fs::{create_dir_all, read_to_string, File};
use std::io::Write;
use std::path::PathBuf;

mod from_binding;
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
                let mut toasts = Toasts::new().anchor(Align2::LEFT_BOTTOM, (-10.0, -10.0));

                DockArea::new(tree)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show_close_buttons(false)
                    .show_add_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .show_leaf_close_all_buttons(false)
                    .show(ctx, views);

                for e in views.error.drain(0..) {
                    toasts.add(Toast {
                        kind: egui_toast::ToastKind::Error,
                        text: e.into(),
                        ..Default::default()
                    });
                }

                toasts.show(ctx);
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

        if update {
            match self.write_out() {
                Ok(_) => {}
                Err(err) => self.add_error(err.to_string()),
            }
        }
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

#[derive(Debug)]
struct Views {
    save_file: PathBuf,

    url: Option<String>,

    commands: BTreeSet<String>,
    command_to_bindings: BTreeMap<String, Vec<Binding>>,

    binding_to_command: BTreeMap<Binding, Vec<String>>,

    manage_tab: managetab::ManageTab,
    from_commands: from_commands::FromCommands,
    from_bindings: from_binding::FromBindings,

    error: Vec<String>,
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

impl Views {
    fn try_add_binding(&mut self, binding: Binding, command: &String) -> bool {
        if self
            .command_to_bindings
            .get(command)
            .unwrap_or(&Vec::new())
            .contains(&binding)
        {
            self.error.push("you already have this binding".to_string());
            false
        } else {
            self.command_to_bindings
                .entry(command.clone())
                .or_insert(Vec::new())
                .push(binding);

            self.binding_to_command
                .entry(binding)
                .or_insert(Vec::new())
                .push(command.clone());
            true
        }
    }

    fn write_out(&self) -> Result<()> {
        // let mut dir = self.directory.clone();

        create_dir_all(self.save_file.parent().unwrap())?;

        let mut file =
            File::create(&self.save_file).with_context(|| "failed to create file to save to")?;

        file.write_all(
            serde_json::to_string(&self.to_bindings())
                .unwrap()
                .as_bytes(),
        )
        .with_context(|| "failed to save to disk")?;

        Ok(())
    }

    fn to_bindings(&self) -> Bindings {
        Bindings {
            url: Cow::Borrowed(&self.url),
            commands: Cow::Borrowed(&self.commands),
            command_to_bindings: Cow::Borrowed(&self.command_to_bindings),
        }
    }

    fn add_error(&mut self, error: String) {
        self.error.push(error);
    }

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
            save_file: path,
            url: bindings.url.into_owned(),
            commands: bindings.commands.into_owned(),
            command_to_bindings: bindings.command_to_bindings.into_owned(),
            binding_to_command,
            manage_tab: ManageTab::default(),
            from_commands: FromCommands::default(),
            error: Vec::new(),
        }
    }

    fn from_directory(mut path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            return Err(ProgramError::NotDirectory(path))?;
            // return format!("{} is not a directory", path.display())?;
        }

        path.push("src");
        path.push("main");
        path.push("deploy");
        path.push("bindings.json");

        if path.is_dir() {
            // return is here just to convice the borrow checker that this path never
            // continues executing the function
            return Err(ProgramError::ExistingDirectoryAt(path))?;
        }

        if !path.exists() {
            return Ok(Self {
                save_file: path,
                url: None,
                commands: BTreeSet::new(),
                command_to_bindings: BTreeMap::new(),
                binding_to_command: BTreeMap::new(),
                manage_tab: ManageTab::default(),
                from_commands: FromCommands::default(),
                error: Vec::new(),
            });
        }

        let file = read_to_string(&path)?;

        let bindings: Bindings = serde_json::from_str(&file)?;

        Ok(Self::from_bindings(bindings, path))
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
