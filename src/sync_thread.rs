// FOR WHEN EXTERNAL EVENT LOOP SUPPORT IS ADDED

use std::{path::PathBuf, process::ExitStatus, time::Instant};

use eframe::UserEvent;
use egui::ViewportId;
use smol::{
    channel::{Receiver, RecvError, Sender},
    future::race,
    process::{Child, Command},
    Executor, Task,
};
use winit::event_loop::EventLoopProxy;

enum SyncCommands {
    UpdateUrl(Option<String>),
    NewSync,
    Quit,
}

enum SyncEvents {
    NewCommand(Result<SyncCommands, RecvError>),
    CommandFinished(Result<ExitStatus, std::io::Error>),
}

fn spawn_thead(
    ev: EventLoopProxy<UserEvent>,
    commands: Receiver<SyncCommands>,
    errors: Sender<String>,
    save_file: PathBuf,
) -> Task<()> {
    let ex = Executor::new();

    ex.spawn(async {
        let mut running = true;
        let mut command: Option<smol::process::Child> = None;
        let mut url = None;

        while (running) {
            let event = match &mut command {
                Some(child) => {
                    race(
                        async { SyncEvents::CommandFinished(child.status().await) },
                        async { SyncEvents::NewCommand(commands.recv().await) },
                    )
                    .await
                }
                None => SyncEvents::NewCommand(commands.recv().await),
            };

            match event {
                SyncEvents::NewCommand(sync_commands) => match sync_commands {
                    Ok(c) => match c {
                        SyncCommands::UpdateUrl(u) => url = u,
                        SyncCommands::NewSync => {
                            kill_command(&mut command);

                            match &url {
                                Some(url) => {
                                    match Command::new("scp")
                                        .arg(&save_file)
                                        .arg(format!("lvuser@{url}:~/deploy/bindings.json"))
                                        .spawn()
                                    {
                                        Ok(child) => command = Some(child),
                                        Err(err) => {
                                            let _ = errors.send(err.to_string()).await;
                                            let _ = ev.send_event(UserEvent::RequestRepaint {
                                                viewport_id: ViewportId::ROOT,
                                                when: Instant::now(),
                                                cumulative_pass_nr: 0,
                                            });
                                        }
                                    }
                                }
                                None => {}
                            }
                        }
                        SyncCommands::Quit => running = false,
                    },
                    Err(_) => running = false,
                },
                SyncEvents::CommandFinished(exit_status) => match exit_status {
                    Ok(status) => {
                        if !status.success() {
                            let _ = errors.send("failed to sync".to_string()).await;
                            command = None;
                            let _ = ev.send_event(UserEvent::RequestRepaint {
                                viewport_id: ViewportId::ROOT,
                                when: Instant::now(),
                                cumulative_pass_nr: 0,
                            });
                        }
                    }
                    Err(err) => {
                        let _ = errors.send(err.to_string()).await;
                        let _ = ev.send_event(UserEvent::RequestRepaint {
                            viewport_id: ViewportId::ROOT,
                            when: Instant::now(),
                            cumulative_pass_nr: 0,
                        });
                    }
                },
            }
        }
    })
}

fn kill_command(child: &mut Option<Child>) {
    match child {
        Some(c) => {
            c.kill().unwrap();
            *child = None;
        }
        None => {}
    }
}
