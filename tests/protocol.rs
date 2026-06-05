//! End-to-end protocol test: the real plugin binary against a mock OpenDeck
//! WebSocket server and a throwaway mosquitto broker. No OpenDeck
//! installation or Stream Deck hardware required.

use futures_util::{SinkExt, StreamExt};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;

const ACTION_UUID: &str = "com.geoffdavis.teamsforlinux.toggle-mute";

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

type WsStream = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

/// Read frames until one matches `predicate`, panicking after 10s.
async fn expect_message<F: Fn(&Value) -> bool>(
    ws: &mut WsStream,
    what: &str,
    predicate: F,
) -> Value {
    timeout(Duration::from_secs(10), async {
        loop {
            let frame = ws.next().await.expect("ws closed").expect("ws error");
            if let Message::Text(text) = frame {
                let value: Value = serde_json::from_str(&text).expect("invalid json from plugin");
                if predicate(&value) {
                    return value;
                }
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
}

async fn expect_title(ws: &mut WsStream, title: &str) {
    expect_message(ws, &format!("setTitle {title}"), |v| {
        v["event"] == "setTitle" && v["payload"]["title"] == title
    })
    .await;
}

fn instance_event(event: &str) -> String {
    json!({
        "event": event,
        "action": ACTION_UUID,
        "context": "ctx1",
        "device": "dev1",
        "payload": {
            "settings": {},
            "controller": "Keypad",
            "state": 0,
            "isInMultiAction": false,
        },
    })
    .to_string()
}

#[tokio::test]
async fn plugin_speaks_protocol_end_to_end() {
    // 1. Throwaway mosquitto broker on a free port.
    let mqtt_port = free_port();
    let dir = tempfile::tempdir().unwrap();
    let conf = dir.path().join("mosquitto.conf");
    std::fs::write(
        &conf,
        format!("listener {mqtt_port} 127.0.0.1\nallow_anonymous true\npersistence false\n"),
    )
    .unwrap();
    let _broker = ChildGuard(
        std::process::Command::new("mosquitto")
            .arg("-c")
            .arg(&conf)
            .spawn()
            .expect("mosquitto not found - enter the dev shell"),
    );
    timeout(Duration::from_secs(10), async {
        while tokio::net::TcpStream::connect(("127.0.0.1", mqtt_port))
            .await
            .is_err()
        {
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("mosquitto did not come up");

    // 2. Config file marks the plugin configured (XDG redirected to the tempdir).
    let xdg = dir.path().join("xdg");
    std::fs::create_dir_all(xdg.join("opendeck-teams-for-linux")).unwrap();
    std::fs::write(
        xdg.join("opendeck-teams-for-linux/config.toml"),
        format!("broker_host = \"127.0.0.1\"\nbroker_port = {mqtt_port}\n"),
    )
    .unwrap();

    // 3. Mock OpenDeck WebSocket server.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ws_port = listener.local_addr().unwrap().port();

    // 4. The real plugin binary.
    let _plugin = ChildGuard(
        std::process::Command::new(env!("CARGO_BIN_EXE_opendeck-teams-for-linux"))
            .args([
                "-port",
                &ws_port.to_string(),
                "-pluginUUID",
                "test-plugin-uuid",
                "-registerEvent",
                "registerPlugin",
                "-info",
                r#"{"devices":[]}"#,
            ])
            .env("XDG_CONFIG_HOME", &xdg)
            .spawn()
            .unwrap(),
    );

    let (stream, _) = timeout(Duration::from_secs(10), listener.accept())
        .await
        .expect("plugin did not connect")
        .unwrap();
    let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();

    // 5. Registration handshake.
    let register = expect_message(&mut ws, "register", |v| v["event"] == "registerPlugin").await;
    assert_eq!(register["uuid"], "test-plugin-uuid");

    // 6. Instance appears; file => configured, call state unknown => OFF.
    ws.send(Message::text(instance_event("willAppear")))
        .await
        .unwrap();
    expect_title(&mut ws, "OFF").await;

    // 7. Drive Teams state over MQTT (retained => plugin subscribe order is irrelevant).
    let mut options = MqttOptions::new("test-driver", "127.0.0.1", mqtt_port);
    options.set_keep_alive(Duration::from_secs(10));
    let (client, mut eventloop) = AsyncClient::new(options, 16);
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(p))) => {
                    let _ = cmd_tx.send(String::from_utf8_lossy(&p.payload).to_string());
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
    client
        .subscribe("teams/command", QoS::AtLeastOnce)
        .await
        .unwrap();

    client
        .publish("teams/in-call", QoS::AtLeastOnce, true, "true")
        .await
        .unwrap();
    expect_title(&mut ws, "MIC").await;

    client
        .publish("teams/microphone/control", QoS::AtLeastOnce, true, "muted")
        .await
        .unwrap();
    expect_title(&mut ws, "MUTED").await;

    client
        .publish(
            "teams/microphone/control",
            QoS::AtLeastOnce,
            true,
            "unmuted",
        )
        .await
        .unwrap();
    expect_title(&mut ws, "MIC").await;

    // 8. Key press while in a call publishes toggle-mute.
    ws.send(Message::text(instance_event("keyDown")))
        .await
        .unwrap();
    let command = timeout(Duration::from_secs(10), cmd_rx.recv())
        .await
        .expect("timed out waiting for toggle-mute")
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&command).unwrap()["action"],
        "toggle-mute"
    );

    // 9. Call ends => OFF; key presses now no-op (plugin sends showAlert, not a publish).
    client
        .publish("teams/in-call", QoS::AtLeastOnce, true, "false")
        .await
        .unwrap();
    expect_title(&mut ws, "OFF").await;
    ws.send(Message::text(instance_event("keyDown")))
        .await
        .unwrap();
    expect_message(&mut ws, "showAlert", |v| v["event"] == "showAlert").await;

    // The guard must not have published anything: give a stray frame time to
    // arrive, then assert the command channel stayed empty.
    sleep(Duration::from_millis(300)).await;
    assert!(
        cmd_rx.try_recv().is_err(),
        "keyDown outside a call must not publish to the command topic"
    );
}
