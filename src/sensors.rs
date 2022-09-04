use futures_util::{future::BoxFuture, FutureExt, StreamExt};
use serde::Deserialize;
use std::{
    fmt::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::TcpStream, time::Instant};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub struct SensorState {
    latest_measurement: Instant,
    sensors: Vec<Sensor>,
}

impl Default for SensorState {
    fn default() -> Self {
        SensorState {
            latest_measurement: Instant::now() - Duration::from_secs(100),
            sensors: vec![],
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Sensor {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "SensorType")]
    sensor_type: String,
    #[serde(rename = "Value")]
    value: f64,
}

#[derive(Deserialize, Debug)]
struct SensorValues {
    #[serde(rename = "Sensors")]
    sensors: Vec<Sensor>,
}

pub fn init(capframex_url: &reqwest::Url) -> Arc<Mutex<SensorState>> {
    let state = Arc::new(Mutex::new(SensorState::default()));
    let mut url = capframex_url.clone();
    url.set_path("/ws/activesensors");
    url.set_scheme("ws").unwrap();
    let url = url.to_string();
    tokio::spawn({
        let state = Arc::clone(&state);
        connect_loop(url, state)
    });
    state
}

fn connect_loop(url: String, state: Arc<Mutex<SensorState>>) -> BoxFuture<'static, ()> {
    async move {
        let c = tokio_tungstenite::connect_async(&url).await;
        let stream = if let Ok((stream, _)) = c {
            stream
        } else {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                connect_loop(url, state).await;
            });
            return;
        };
        let _ = consume_loop(stream, Arc::clone(&state)).await;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            connect_loop(url, state).await;
        });
    }
    .boxed()
}

async fn consume_loop(
    mut stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    state: Arc<Mutex<SensorState>>,
) -> tokio_tungstenite::tungstenite::Result<()> {
    loop {
        let frame = stream.next().await;
        let frame = if let Some(frame) = frame {
            frame?
        } else {
            return Ok(());
        };
        let msg = frame.into_text()?;
        if let Ok(new_values) = serde_json::from_str::<SensorValues>(&msg) {
            let mut guard = state.lock().unwrap();
            guard.sensors = new_values.sensors;
            guard.latest_measurement = Instant::now();
        }
    }
}

pub fn output(output: &mut String, state: &Mutex<SensorState>) {
    writeln!(
        output,
        "# HELP capframex_sensor Hardware sensor tracked by CapFrameX."
    )
    .unwrap();
    writeln!(output, "# TYPE capframex_sensor gauge").unwrap();

    let guard = state.lock().unwrap();

    // only output fresh readouts
    if Instant::now().duration_since(guard.latest_measurement) > Duration::from_secs(15) {
        return;
    }

    for sensor in guard.sensors.iter() {
        writeln!(
            output,
            "capframex_sensor {{name=\"{}\", type=\"{}\"}} {}",
            &sensor.name, &sensor.sensor_type, sensor.value
        )
        .unwrap();
    }
}
