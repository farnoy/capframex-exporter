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
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mut capframex_url = reqwest::Url::parse("http://localhost").unwrap();
    capframex_url.set_ip_host(args.capframex_url.ip()).unwrap();
    capframex_url
        .set_port(Some(args.capframex_url.port()))
        .unwrap();

    let capframex_client = reqwest::Client::builder().build().unwrap();

    let handler = warp::path("metrics")
        .and(warp::any().map(move || capframex_client.clone()))
        .then(move |capframex_client: reqwest::Client| {
            let x = async move {
                let processes = async {
                    Ok(capframex_client
                        .get(&format!("http://{}/api/processes", args.capframex_url))
                        .send()
                        .await?
                        .json()
                        .await?)
                };
                let metrics = async {
                    Ok(capframex_client
                        .get(&format!(
                            "http://{}/api/metrics?metricNames=P95,Average",
                            args.capframex_url
                        ))
                        .send()
                        .await?
                        .json::<(f32, f32)>()
                        .await
                        .map(|a| Some(a))
                        .unwrap_or(None))
                };
                let (processes, metrics): (
                    reqwest::Result<Vec<String>>,
                    reqwest::Result<Option<(f32, f32)>>,
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
                if let Some((p95, avg)) = metrics {
                    writeln!(output, "capframex_fps {{name=\"p95\"}} {}", p95).unwrap();
                    writeln!(output, "capframex_fps {{name=\"avg\"}} {}", avg).unwrap();
                }
                Ok(output)
                // Ok(format!("Hello {processes:?} {metrics:?}"))
            };
            x.unwrap_or_else(|x: reqwest::Error| {
                dbg!(x);
                "err".to_string()
            })
        });

    warp::serve(handler).run(args.bind_address).await;
}
