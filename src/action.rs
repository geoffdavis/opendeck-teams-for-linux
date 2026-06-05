//! OpenAction integration: the toggle-mute action and display pushing.

use crate::mqtt::{DisplayInput, MqttController};
use crate::state::{Display, Icon};

use base64::Engine as _;
use openaction::{Action, Instance, OpenActionResult, async_trait, visible_instances};
use std::sync::{Arc, LazyLock};
use tokio::sync::watch;

fn png_data_uri(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

static ICON_NORMAL: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/icon@2x.png")));
static ICON_MUTED: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/icon-muted@2x.png")));
static ICON_OFF: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/icon-off@2x.png")));

fn icon_data_uri(icon: Icon) -> &'static str {
    match icon {
        Icon::Normal => &ICON_NORMAL,
        Icon::Muted => &ICON_MUTED,
        Icon::Off => &ICON_OFF,
    }
}

pub async fn push_display(instance: &Instance, display: Display) -> OpenActionResult<()> {
    instance.set_state(display.state_index).await?;
    instance.set_title(Some(display.title), None).await?;
    instance
        .set_image(Some(icon_data_uri(display.icon)), None)
        .await
}

/// Watch for state changes and refresh every visible instance.
pub fn spawn_display_pusher(mut display_rx: watch::Receiver<DisplayInput>) {
    tokio::spawn(async move {
        while display_rx.changed().await.is_ok() {
            let input = *display_rx.borrow_and_update();
            let display = crate::state::display(input.mic, input.configured);
            for instance in visible_instances(ToggleMuteAction::UUID).await {
                if let Err(err) = push_display(&instance, display).await {
                    log::warn!("display push failed: {err}");
                }
            }
        }
    });
}

pub struct ToggleMuteAction {
    pub controller: Arc<MqttController>,
}

#[async_trait]
impl Action for ToggleMuteAction {
    const UUID: &'static str = "com.geoffdavis.teamsforlinux.toggle-mute";
    type Settings = crate::settings::PiSettings;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.controller.apply_settings(settings).await;
        push_display(instance, self.controller.current_display()).await
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.controller.apply_settings(settings).await;
        push_display(instance, self.controller.current_display()).await
    }

    async fn key_down(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if !self.controller.key_pressed().await {
            // Not in a call / unconfigured / publish failure: flash the warning triangle.
            instance.show_alert().await?;
        }
        Ok(())
    }

    async fn property_inspector_did_appear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        // Report effective values so the PI can show them as placeholders.
        // Deliberate exposure: broker host/username go to the PI form over the
        // local OpenDeck WebSocket only (not logged, not persisted by us);
        // the password itself is never sent, only a set/unset flag.
        if let Some(resolved) = self.controller.resolved().await {
            instance
                .send_to_property_inspector(serde_json::json!({
                    "event": "effectiveSettings",
                    "broker_host": resolved.broker_host,
                    "broker_port": resolved.broker_port.to_string(),
                    "username": resolved.username,
                    "password_set": !resolved.password.is_empty(),
                    "topic_prefix": resolved.topic_prefix,
                    "microphone_topic": resolved.microphone_suffix,
                    "microphone_control_topic": resolved.microphone_control_suffix,
                    "in_call_topic": resolved.in_call_suffix,
                    "command_topic": resolved.command_suffix,
                }))
                .await?;
        }
        Ok(())
    }
}
