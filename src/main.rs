use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use futures_signals::signal::Mutable;
use futures_signals::signal_map::MutableBTreeMap;
use futures_signals::signal_vec::MutableVec;
use gtk4::{prelude::*, ApplicationWindow, Box, Widget};
use gtk4::{glib, Application};
use serde::{Deserialize, Serialize};

const APP_ID: &str = "xyz.staugaard.bindings-gui";


#[derive(Debug,PartialEq,Eq,Hash,PartialOrd,Ord,Serialize,Deserialize)]
enum Button {
    Button(u8),
    Pov(u8),
}

#[derive(Debug,PartialEq,Eq,Hash,PartialOrd,Ord,Serialize,Deserialize)]
struct Binding {
    controller: u8,
    button: Button,
}

#[derive(Debug,Serialize,Deserialize)]
enum App {
    Initial,

    Started {
        directory: Mutable<PathBuf>,

        url: Mutable<String>,
    
        commands: Mutable<HashSet<String>>,
        command_to_bindings: MutableBTreeMap<String, MutableVec<Binding>>,
        binding_to_command: MutableBTreeMap<Binding, String>,
    }
}

impl App {
    fn renderWidget(s: Mutable<Self>) -> Widget {
        match s.lock_ref().deref() {
            App::Initial => Box::new(gtk4::Orientation::Vertical, 0).into(),
            App::Started { 
                directory, 
                url, 
                commands, 
                command_to_bindings, 
                binding_to_command 
            } => todo!(),
        }
    }

    fn renderBody(&self) -> Widget {
        todo!()

    }

    fn render(o: Mutable<Self>, app: &Application) -> () {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("My GTK App")
            .child(&o.lock_mut().deref().renderBody())
            .build();

        o.signal_ref(|app| {
            window.set_child(Some(&app.renderBody()));
            async {}
        }
        );

        window.present();
    }
}

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    let state = Mutable::new(App::Initial);

    app.connect_activate(|app| {
        App::render(state, app);

    });


    app.run()
}
