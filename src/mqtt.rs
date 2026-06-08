//! MQTT connection management: subscribe to state topics, publish commands.

use crate::settings::{self, PiSettings, Resolved};
use crate::state::{MicState, StateTopic};

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;

/// Everything the display pusher needs to render the button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DisplayInput {
    pub mic: MicState,
    pub configured: bool,
}

#[derive(Default)]
struct Inner {
    resolved: Option<Resolved>,
    client: Option<AsyncClient>,
    task: Option<JoinHandle<()>>,
}

pub struct MqttController {
    display_tx: watch::Sender<DisplayInput>,
    inner: Mutex<Inner>,
}

impl MqttController {
    pub fn new() -> (Arc<Self>, watch::Receiver<DisplayInput>) {
        let (display_tx, display_rx) = watch::channel(DisplayInput::default());
        let controller = Arc::new(Self {
            display_tx,
            inner: Mutex::new(Inner::default()),
        });
        (controller, display_rx)
    }

    /// Latest Teams state + configured flag, for a control to render from.
    pub fn current_input(&self) -> DisplayInput {
        *self.display_tx.borrow()
    }

    /// Currently resolved settings (for PI placeholder reporting).
    pub async fn resolved(&self) -> Option<Resolved> {
        self.inner.lock().await.resolved.clone()
    }

    /// Re-resolve settings (file + PI) and reconnect if anything changed.
    pub async fn apply_settings(&self, pi: &PiSettings) {
        let file = settings::load_file_settings(&settings::config_file_path());
        let resolved = settings::resolve(file.as_ref(), pi);

        let mut inner = self.inner.lock().await;
        if inner.resolved.as_ref() == Some(&resolved) {
            return;
        }
        log::info!("settings changed; reconnecting mqtt");
        if let Some(task) = inner.task.take() {
            task.abort();
        }
        inner.client = None;

        // Reset state and propagate the configured flag; notifies the pusher.
        self.display_tx.send_modify(|d| {
            d.mic = MicState::default();
            d.configured = resolved.configured;
        });

        if resolved.configured {
            let (client, task) = spawn_mqtt(resolved.clone(), self.display_tx.clone());
            inner.client = Some(client);
            inner.task = Some(task);
        }
        inner.resolved = Some(resolved);
    }

    /// Handle a key press for a control. Publishes `command` to the command
    /// topic when `can_activate` allows it for the current state. Returns true
    /// if the command was published.
    pub async fn key_pressed(
        &self,
        command: &str,
        can_activate: fn(MicState, bool) -> bool,
    ) -> bool {
        let (command_topic, client) = {
            let inner = self.inner.lock().await;
            let input = *self.display_tx.borrow();
            if !can_activate(input.mic, input.configured) {
                log::info!("ignoring key press: control not active for the current Teams state");
                return false;
            }
            let (Some(resolved), Some(client)) = (&inner.resolved, &inner.client) else {
                return false;
            };
            (resolved.command_topic(), client.clone())
        }; // inner guard dropped here — publish must not hold the lock
        match client
            .publish(command_topic, QoS::AtLeastOnce, false, command.as_bytes())
            .await
        {
            Ok(()) => {
                log::info!("published command: {command}");
                true
            }
            Err(err) => {
                log::error!("mqtt publish failed: {err}");
                false
            }
        }
    }
}

/// Spawn the connection task: subscribe on connect, fold publishes into state,
/// retry forever with a fixed 5s delay on errors.
fn spawn_mqtt(
    resolved: Resolved,
    display_tx: watch::Sender<DisplayInput>,
) -> (AsyncClient, JoinHandle<()>) {
    let mut options = MqttOptions::new(
        "opendeck-teams-for-linux",
        resolved.broker_host.clone(),
        resolved.broker_port,
    );
    options.set_keep_alive(Duration::from_secs(30));
    if !resolved.username.is_empty() {
        options.set_credentials(resolved.username.clone(), resolved.password.clone());
    }

    let (client, mut eventloop) = AsyncClient::new(options, 16);
    let subscriptions = [
        (resolved.microphone_topic(), StateTopic::Microphone),
        (
            resolved.microphone_control_topic(),
            StateTopic::MicrophoneControl,
        ),
        (resolved.in_call_topic(), StateTopic::InCall),
    ];

    let subscribe_client = client.clone();
    let task = tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    log::info!("mqtt connected");
                    for (topic, _) in &subscriptions {
                        if let Err(err) = subscribe_client
                            .subscribe(topic.clone(), QoS::AtLeastOnce)
                            .await
                        {
                            log::error!("mqtt subscribe failed: {err}");
                        }
                    }
                }
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    let payload = String::from_utf8_lossy(&publish.payload)
                        .trim()
                        .to_lowercase();
                    log::debug!("mqtt rx {}={payload}", publish.topic);
                    if let Some((_, topic)) =
                        subscriptions.iter().find(|(t, _)| *t == publish.topic)
                    {
                        display_tx.send_if_modified(|d| d.mic.apply(*topic, &payload));
                    }
                }
                Ok(_) => {}
                Err(err) => {
                    log::warn!("mqtt error: {err}; retrying in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    (client, task)
}
