use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream};

pub type WebSocketWrite =
    SplitSink<tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

pub type WebSocketRead = SplitStream<tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub type WebSocketStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;
