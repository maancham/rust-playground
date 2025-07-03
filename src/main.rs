use clap::Parser;
use tokio::net::TcpListener;

mod utils;
mod response;
mod router;
mod connection;

use connection::handle_connection;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "./tmp")]
    directory: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let file_directory = args.directory.clone();
        tokio::spawn(async move {
            if let Err(_) = handle_connection(stream, file_directory).await {
                println!("client closed connection");
                return;
            }
        });
    }
}
