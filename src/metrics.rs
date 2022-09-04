use std::fmt::Write;

pub async fn get(
    capframex_client: &reqwest::Client,
    capframex_url: &reqwest::Url,
    metric_names: &[String],
) -> reqwest::Result<Vec<f32>> {
    Ok(capframex_client
        .get(
            capframex_url
                .join(&format!(
                    "/api/metrics?metricNames={}",
                    metric_names.join(",")
                ))
                .unwrap(),
        )
        .send()
        .await?
        .json::<Vec<f32>>()
        .await
        .unwrap_or_default())
}

pub fn output(output: &mut String, metric_names: &[String], metrics: &[f32]) {
    writeln!(
        output,
        "# HELP capframex_fps Performance metric tracked by CapFrameX."
    )
    .unwrap();
    writeln!(output, "# TYPE capframex_fps gauge").unwrap();
    for (metric, value) in metric_names.iter().zip(metrics.iter()) {
        writeln!(output, "capframex_fps {{name=\"{}\"}} {}", metric, value).unwrap();
    }
}
