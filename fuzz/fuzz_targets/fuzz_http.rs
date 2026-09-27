//! Fuzz through the full HTTP surface: the fuzz bytes become the body of a
//! multipart `POST /import` against the real router, so multipart parsing,
//! body limits, CORS/trace layers, and error mapping are all covered by the
//! same corpus.

#![no_main]

use http_body_util::BodyExt;
use libfuzzer_sys::fuzz_target;
use tower::ServiceExt;

const BOUNDARY: &str = "XfuzzboundaryX";

fuzz_target!(|data: &[u8]| {
    let mut body: Vec<u8> = Vec::with_capacity(data.len() + 160);
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"fuzz.fig\"\r\n\
             Content-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("fuzz runtime");

    runtime.block_on(async move {
        let request = axum::http::Request::builder()
            .method(axum::http::Method::POST)
            .uri("/import")
            .header(
                axum::http::header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(axum::body::Body::from(body))
            .expect("static request");

        let app = figma_import::build_router();
        let response = app.oneshot(request).await.expect("infallible response");
        // Drain so axum/tracing warning paths run too.
        let _ = response.into_body().collect().await;
    });
});
