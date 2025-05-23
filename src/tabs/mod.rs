use std::collections::BTreeSet;

use constants::ConstantsTab;
use driver_constants::DriverConstantsTab;
use from_bindings::FromBindings;
use from_commands::FromCommands;
use manage_commands::ManageTab;
use manage_controllers::ManageControllers;
use manage_streams::ManageStreamsTab;
use once_cell::sync::Lazy;
use profiles::ProfilesTab;
use streams::StreamsTab;
use syncing::SyncingTab;

use crate::{
    global_state::{GlobalEvents, State},
    Component,
};

pub mod constants;
pub mod driver_constants;
pub mod from_bindings;
pub mod from_commands;
pub mod manage_commands;
pub mod manage_controllers;
pub mod manage_streams;
pub mod password_lock;
pub mod profiles;
pub mod streams;
pub mod syncing;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) enum TabType {
    Constants,
    DriverConstants,
    FromBindings,
    FromCommands,
    ManageCommands,
    ManageControllers,
    ManageSteams,
    Profiles,
    Streams,
    Syncing,
}

pub static ALL_TABS: Lazy<BTreeSet<TabType>> = Lazy::new(|| {
    BTreeSet::from_iter([
        TabType::Constants,
        TabType::DriverConstants,
        TabType::FromBindings,
        TabType::FromCommands,
        TabType::ManageCommands,
        TabType::ManageControllers,
        TabType::ManageSteams,
        TabType::Profiles,
        TabType::Streams,
        TabType::Syncing,
    ])
});

impl TabType {
    pub fn name(&self) -> &'static str {
        match self {
            TabType::Constants => "constants",
            TabType::DriverConstants => "driver constants",
            TabType::FromBindings => "from bindings",
            TabType::FromCommands => "from commands",
            TabType::ManageCommands => "manage commands",
            TabType::ManageControllers => "manage controllers",
            TabType::ManageSteams => "manage streams",
            TabType::Profiles => "manage profiles",
            TabType::Streams => "streams",
            TabType::Syncing => "syncing",
        }
    }

    pub fn build(&self) -> Box<dyn Component<OutputEvents = GlobalEvents, Environment = State>> {
        match self {
            TabType::Constants => Box::new(ConstantsTab::default().lock()),
            TabType::DriverConstants => Box::new(DriverConstantsTab::default()),
            TabType::FromBindings => Box::new(FromBindings::default()),
            TabType::FromCommands => Box::new(FromCommands::default()),
            TabType::ManageCommands => Box::new(ManageTab::default().lock()),
            TabType::ManageControllers => Box::new(ManageControllers::default()),
            TabType::ManageSteams => Box::new(ManageStreamsTab::default().lock()),
            TabType::Profiles => Box::new(ProfilesTab::default()),
            TabType::Streams => Box::new(StreamsTab::default()),
            TabType::Syncing => Box::new(SyncingTab::default().lock()),
        }
    }
}
