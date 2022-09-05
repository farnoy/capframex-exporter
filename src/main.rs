use clap::Parser;
use futures_util::{join, TryFutureExt};

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use warp::Filter;

mod metrics;
mod processes;
mod sensors;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, default_value = "0.0.0.0:9032")]
    bind_address: SocketAddr,

    #[clap(short, default_value = "127.0.0.1:1337")]
    capframex_url: SocketAddr,

    #[clap(short, default_value = "Average,P1,P0dot2")]
    metrics: Vec<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let Args {
        bind_address,
        capframex_url,
        metrics,
    } = Args::parse();

    let capframex_url = {
        let mut url = reqwest::Url::parse("http://localhost").unwrap();
        url.set_ip_host(capframex_url.ip()).unwrap();
        url.set_port(Some(capframex_url.port())).unwrap();
        Arc::new(url)
    };

    let metric_names = Arc::new(metrics);
    let capframex_client = Arc::new(reqwest::Client::builder().build().unwrap());
    let sensor_state = sensors::init(&capframex_url);

    let handler = warp::path("metrics").then({
        let capframex_client = Arc::clone(&capframex_client);
        let capframex_url = Arc::clone(&capframex_url);
        let metric_names = Arc::clone(&metric_names);
        let sensor_state = Arc::clone(&sensor_state);

        move || {
            let capframex_client = Arc::clone(&capframex_client);
            let capframex_url = Arc::clone(&capframex_url);
            let metric_names = Arc::clone(&metric_names);
            let sensor_state = Arc::clone(&sensor_state);
            metrics_handler(capframex_client, capframex_url, metric_names, sensor_state)
        }
    });

    warp::serve(handler).run(bind_address).await;
}

async fn metrics_handler(
    capframex_client: Arc<reqwest::Client>,
    capframex_url: Arc<reqwest::Url>,
    metric_names: Arc<Vec<String>>,
    sensor_state: Arc<Mutex<sensors::SensorState>>,
) -> String {
    let x = async move {
        let processes = processes::get(&capframex_client, &capframex_url);
        let metrics = metrics::get(&capframex_client, &capframex_url, &metric_names);
        let (processes, metrics): (reqwest::Result<Vec<String>>, reqwest::Result<Vec<f32>>) =
            join!(processes, metrics);
        let processes = processes.unwrap_or_default();
        let metrics = metrics.unwrap_or_default();
        let mut output = String::new();
        processes::output(&mut output, &processes);
        metrics::output(&mut output, &metric_names, &metrics);
        sensors::output(&mut output, &sensor_state);
        Ok(output)
        // Ok(format!("Hello {processes:?} {metrics:?}"))
    };
    x.unwrap_or_else(|x: reqwest::Error| {
        dbg!(x);
        "err".to_string()
    })
    .await
}
