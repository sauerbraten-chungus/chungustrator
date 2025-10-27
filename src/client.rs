use chungus::Meow;
use chungus::chungus_enet_client::ChungusEnetClient;

pub mod chungus {
    tonic::include_proto!("chungus");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ChungusEnetClient::connect("http://127.0.0.1:50051").await?;

    let request = tonic::Request::new(Meow {
        data: "bruh".into(),
    });

    let response = client.hello(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
