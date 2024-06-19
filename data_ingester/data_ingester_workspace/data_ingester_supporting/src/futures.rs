pub(crate) async fn try_collect_send<T>(
    name: &str,
    future: impl Future<Output = Result<T>>,
    splunk: &Splunk,
) -> Result<()>
where
    for<'a> &'a T: ToHecEvents + Debug,
{
    info!("Getting {}", &name);
    match future.await {
        Ok(ref result) => {
            let hec_events = match result.to_hec_events() {
                Ok(hec_events) => hec_events,
                Err(e) => {
                    error!("Failed converting to HecEvents: {}", e);
                }
            };

            match splunk.send_batch(&hec_events).await {
                Ok(_) => debug!("Sent to Splunk"),
                Err(e) => {
                    error!("Failed Sending to Splunk: {}", e);
                }
            };
        }
        Err(err) => {
            error!("Failed to get {}: {}", &name, err);
        }
    };
    Ok(())
}
