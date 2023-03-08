use serde_json::Value;
use std::fmt::Write;
use std::str::FromStr;

pub async fn get(
    capframex_client: &reqwest::Client,
    capframex_url: &reqwest::Url,
    metric_names: &[String],
) -> reqwest::Result<Vec<f32>> {
    let results = capframex_client
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
        .json::<Vec<Value>>()
        .await?;

    let parsed = results
        .into_iter()
        .map(|value| match value {
            Value::String(s) => f32::from_str(&s).ok(),
            Value::Number(num) => num.as_f64().map(|double| double as f32),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();

    Ok(parsed)
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
