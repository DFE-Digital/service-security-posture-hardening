use futures::TryStreamExt;
use tiberius::{AuthMethod, Client, Config, Query, QueryItem};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

#[tokio::test]
async fn main() -> anyhow::Result<()> {
    let mut config = Config::new();

    config.host("host");
    config.port(1433);
    config.authentication(AuthMethod::sql_server("username", "password"));
    config.database("db");
    config.encryption(tiberius::EncryptionLevel::Required);

    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;

    let mut client = Client::connect(config, tcp.compat_write()).await?;
    let mut select = Query::new("SELECT * FROM table");
    let mut stream = select.query(&mut client).await?;
    while let Some(item) = stream.try_next().await? {
        match item {
            // our first item is the column data always
            QueryItem::Metadata(meta) => {
                dbg!(&meta);
            }
            // ... and from there on from 0..N rows
            QueryItem::Row(row) => {
                dbg!(&row);
            }
        }
    }
    Ok(())
}
