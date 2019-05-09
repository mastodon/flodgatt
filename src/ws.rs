use crate::stream::StreamManager;
use futures::future::Future;
use futures::stream::Stream;
use futures::Async;

pub fn handle_ws(
    socket: warp::ws::WebSocket,
    mut stream: StreamManager,
) -> impl futures::future::Future<Item = (), Error = ()> {
    let (tx, rx) = futures::sync::mpsc::unbounded();
    let (ws_tx, mut ws_rx) = socket.split();
    warp::spawn(
        rx.map_err(|()| -> warp::Error { unreachable!() })
            .forward(ws_tx)
            .map_err(|_| ())
            .map(|_r| ()),
    );
    let event_stream = tokio::timer::Interval::new(
        std::time::Instant::now(),
        std::time::Duration::from_millis(100),
    )
    .take_while(move |_| {
        if ws_rx.poll().is_err() {
            futures::future::ok(false)
        } else {
            futures::future::ok(true)
        }
    });

    event_stream
        .for_each(move |_json_value| {
            if let Ok(Async::Ready(Some(json_value))) = stream.poll() {
                let msg = warp::ws::Message::text(json_value.to_string());
                if !tx.is_closed() {
                    tx.unbounded_send(msg).expect("No send error");
                }
            };
            Ok(())
        })
        .then(|msg| msg)
        .map_err(|e| println!("{}", e))
}
