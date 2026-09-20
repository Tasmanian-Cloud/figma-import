//! End-to-end test: drives the real Axum router (in-process, via
//! `tower::ServiceExt::oneshot`) with a multipart upload carrying the
//! real binary `.fig` fixture from `figma_import::fixtures`, and asserts
//! on the actual JSON response — not a mocked parse result.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn multipart_body(field_name: &str, file_name: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    let boundary = "figma-import-test-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{field_name}\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

#[tokio::test]
async fn health_reports_ok() {
    let app = figma_import::build_router();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn import_parses_the_real_sample_fig_into_a_pen_document() {
    let fig_bytes = figma_import::fixtures::sample_fig_bytes();
    let (content_type, body) = multipart_body("file", "sample.fig", &fig_bytes);

    let app = figma_import::build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/import")
                .header("content-type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid JSON response");

    assert_eq!(json["ok"], true);

    // The real op-figma-converted PenDocument, not a synthesized stand-in:
    // one page ("Landing Page"), whose Hero frame carries the Headline
    // text node and the CTA Button rectangle.
    let pages = json["document"]["pages"]
        .as_array()
        .expect("document has a pages array");
    assert_eq!(pages.len(), 1, "one user page (CANVAS) in the fixture");
    assert_eq!(pages[0]["name"], "Landing Page");

    let summary = &json["summary"];
    assert_eq!(summary["pageCount"], 1);
    assert!(
        summary["totalNodeCount"].as_u64().unwrap() >= 3,
        "hero frame + text + rectangle should all be counted"
    );

    let node_counts = summary["nodeCountsByType"]
        .as_object()
        .expect("nodeCountsByType object");
    assert_eq!(node_counts.get("text").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(
        node_counts.get("rectangle").and_then(|v| v.as_u64()),
        Some(1)
    );

    // The real text_mapper.rs decode path: "textData.characters" round-
    // tripped through Kiwi decode -> TextContent::Plain -> our JSON walk.
    let texts = summary["textContent"]
        .as_array()
        .expect("textContent array");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "Ship faster with Tasmanian Cloud");

    // Unsupported-feature disclosure is always present, not conditional.
    let unsupported = summary["unsupportedFeatures"]
        .as_array()
        .expect("unsupportedFeatures array");
    assert!(!unsupported.is_empty());
}

#[tokio::test]
async fn import_rejects_unrecognised_bytes() {
    let (content_type, body) = multipart_body("file", "not-a-fig.bin", b"hello world");
    let app = figma_import::build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/import")
                .header("content-type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "unknown_format");
}

#[tokio::test]
async fn import_without_a_file_field_is_rejected() {
    let app = figma_import::build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/import")
                .header("content-type", "multipart/form-data; boundary=x")
                .body(Body::from("--x--\r\n"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
