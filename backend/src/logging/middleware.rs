use std::time::Instant;

use axum::middleware::Next;
use hyper::Request;
use tracing::info;

pub (crate) async fn logging_middleware(
  req: Request<axum::body::Body>,
  next: Next
) -> impl axum::response::IntoResponse {
  let path = req.uri().path().to_owned();
  let method = req.method().clone();

  info!("Request received: {} {}", method, path);

  let start = Instant::now();
  let response = next.run(req).await;
  let duration = start.elapsed();

  info!("Request completed: {} {} - {:?}", method, path, duration);
  response
}
