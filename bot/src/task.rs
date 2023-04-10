/// Spawn the given task in the background.
pub(crate) async fn spawn<F>(future: F) -> anyhow::Result<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future).await?
}
