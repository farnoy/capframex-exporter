use clap::Parser;
use futures_util::{join, Future, TryFutureExt};
use std::fmt::Write;
use std::net::SocketAddr;
use warp::Filter;

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

#[tokio::main]
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
        url
    };

    let capframex_client = reqwest::Client::builder().build().unwrap();

    let handler = warp::path("metrics")
        .and(warp::any().map(move || capframex_client.clone()))
        .and(warp::any().map(move || capframex_url.clone()))
        .and(warp::any().map(move || metrics.clone()))
        .then(move |capframex_client: reqwest::Client, capframex_url: reqwest::Url, metric_names: Vec<String>| {
            let x = async move {
                let processes = async {
                    Ok(capframex_client
                        .get(capframex_url.join("/api/processes").unwrap())
                        .send()
                        .await?
                        .json()
                        .await?)
                };
                let metrics = async {
                    Ok(capframex_client
                        .get(capframex_url.join(&format!("/api/metrics?metricNames={}", metric_names.join(","))).unwrap())
                        .send()
                        .await?
                        .json::<Vec<f32>>()
                        .await
                        .unwrap_or(vec![]))
                };
                let (processes, metrics): (
                    reqwest::Result<Vec<String>>,
                    reqwest::Result<Vec<f32>>,
                ) = join!(processes, metrics);
                let (processes, metrics) = (processes?, metrics?);
                let mut output = String::new();
                writeln!(
                    output,
                    "# HELP capframex_active_process Process currently being monitored by CapFrameX."
                )
                .unwrap();
                writeln!(output, "# TYPE capframex_active_process gauge").unwrap();
                for process in processes.iter() {
                    writeln!(output, "capframex_active_process{{name=\"{}\"}} 1", process).unwrap();
                }
                writeln!(
                    output,
                    "# HELP capframex_fps Performance metric tracked by CapFrameX."
                )
                .unwrap();
                writeln!(output, "# TYPE capframex_fps gauge").unwrap();
                for (metric, value) in metric_names.iter().zip(metrics.iter()) {
                    writeln!(output, "capframex_fps {{name=\"{}\"}} {}", metric, value).unwrap();
                }
                Ok(output)
                // Ok(format!("Hello {processes:?} {metrics:?}"))
            };
            x.unwrap_or_else(|x: reqwest::Error| {
                dbg!(x);
                "err".to_string()
            })
        });

    warp::serve(handler).run(bind_address).await;
}
