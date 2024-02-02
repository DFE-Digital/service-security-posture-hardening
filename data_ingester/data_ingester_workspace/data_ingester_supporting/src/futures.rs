pub(crate) async fn try_collect_send<T>(
    name: &str,
    future: impl Future<Output = Result<T>>,
    splunk: &Splunk,
) -> Result<()>
where
    for<'a> &'a T: ToHecEvents + Debug,
{
    splunk.log(&format!("Getting {}", &name)).await?;
    match future.await {
        Ok(ref result) => {
            let hec_events = match result.to_hec_events() {
                Ok(hec_events) => hec_events,
                Err(e) => {
                    eprintln!("Failed converting to HecEvents: {}", e);
                    dbg!(&result);
                    vec![HecEvent::new(
                        &Message {
                            event: format!("Failed converting to HecEvents: {}", e),
                        },
                        "data_ingester_rust",
                        "data_ingester_rust_logs",
                    )?]
                }
            };

            match splunk.send_batch(&hec_events).await {
                Ok(_) => eprintln!("Sent to Splunk"),
                Err(e) => {
                    eprintln!("Failed Sending to Splunk: {}", e);
                    //dbg!(&hec_events);
                }
            };
        }
        Err(err) => {
            splunk
                .log(&format!("Failed to get {}: {}", &name, err))
                .await?
        }
    };
    Ok(())
}
