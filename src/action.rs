//! OpenAction integration: a generic toggle-control action and display pushing.
//!
//! [`ToggleControlAction`] is parameterised by a [`Control`], so every Teams
//! action (mute today; camera/hand/blur later) shares one implementation. New
//! controls are wired up in [`register_controls`].

use crate::control::{Control, MuteControl};
use crate::mqtt::{DisplayInput, MqttController};
use crate::state::Display;

use openaction::{
    Action, Instance, OpenActionResult, async_trait, register_action, visible_instances,
};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::watch;

/// Push one control's display (state index, title, image) to a single key.
pub async fn push_display<C: Control>(
    instance: &Instance,
    display: Display,
) -> OpenActionResult<()> {
    instance.set_state(display.state_index).await?;
    instance.set_title(Some(display.title), None).await?;
    instance
        .set_image(Some(C::icon_data_uri(display.icon)), None)
        .await
}

/// Watch for state changes and refresh every visible instance of control `C`.
pub fn spawn_display_pusher<C: Control>(mut display_rx: watch::Receiver<DisplayInput>) {
    tokio::spawn(async move {
        while display_rx.changed().await.is_ok() {
            let input = *display_rx.borrow_and_update();
            let display = C::display(input.mic, input.configured);
            for instance in visible_instances(C::UUID).await {
                if let Err(err) = push_display::<C>(&instance, display).await {
                    log::warn!("display push failed: {err}");
                }
            }
        }
    });
}

/// Register every control with OpenAction. This is the single place a new
/// toggle control is added to the plugin.
pub async fn register_controls() {
    // Each control currently shares the mic/in-call state from its own
    // connection; a shared MQTT connection across controls is tracked
    // separately. For now there is one control.
    let (controller, display_rx) = MqttController::new();
    spawn_display_pusher::<MuteControl>(display_rx);
    register_action(ToggleControlAction::<MuteControl>::new(controller)).await;
}

/// Generic OpenDeck action for a [`Control`]: publishes the control's command
/// on press and mirrors live Teams state onto the key.
pub struct ToggleControlAction<C: Control> {
    controller: Arc<MqttController>,
    _control: PhantomData<C>,
}

impl<C: Control> ToggleControlAction<C> {
    pub fn new(controller: Arc<MqttController>) -> Self {
        Self {
            controller,
            _control: PhantomData,
        }
    }

    async fn push_current(&self, instance: &Instance) -> OpenActionResult<()> {
        let input = self.controller.current_input();
        push_display::<C>(instance, C::display(input.mic, input.configured)).await
    }
}

#[async_trait]
impl<C: Control> Action for ToggleControlAction<C> {
    const UUID: &'static str = C::UUID;
    type Settings = crate::settings::PiSettings;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.controller.apply_settings(settings).await;
        self.push_current(instance).await
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.controller.apply_settings(settings).await;
        self.push_current(instance).await
    }

    async fn key_down(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if !self
            .controller
            .key_pressed(C::COMMAND, C::can_activate)
            .await
        {
            // The press was rejected by the control's guard or the publish
            // failed: flash the warning triangle.
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
