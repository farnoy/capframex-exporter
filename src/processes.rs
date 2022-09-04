use std::fmt::Write;

pub async fn get(
    capframex_client: &reqwest::Client,
    capframex_url: &reqwest::Url,
) -> reqwest::Result<Vec<String>> {
    capframex_client
        .get(capframex_url.join("/api/processes").unwrap())
        .send()
        .await?
        .json()
        .await
}

pub fn output(output: &mut String, processes: &Vec<String>) {
    writeln!(
        output,
        "# HELP capframex_active_process Process currently being monitored by CapFrameX."
    )
    .unwrap();
    writeln!(output, "# TYPE capframex_active_process gauge").unwrap();
    for process in processes.iter() {
        writeln!(output, "capframex_active_process{{name=\"{}\"}} 1", process).unwrap();
    }
}
