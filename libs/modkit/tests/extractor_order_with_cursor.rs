#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    Router,
    body::to_bytes,
    http::{Request, StatusCode},
    routing::get,
};
use modkit::api::odata::OData;
use tower::ServiceExt;

#[tokio::test]
async fn order_with_cursor_is_400() {
    // trivial route just to trigger extractor
    async fn handler(OData(_q): OData) -> &'static str {
        "ok"
    }

    let app = Router::new().route("/", get(handler));

    // Provide both cursor and $orderby
    let req = Request::builder()
        .uri("/?cursor=eyJ2IjoxLCJrIjpbIjEiXS&$orderby=id%20desc")
        .body(axum::body::Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // Canonical `InvalidArgument` is 400 — replaces the legacy 422 wire status.
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Check body mentions cursor/orderby conflict via the canonical
    // `field_violations` context (field == "cursor", reason "ORDER_WITH_CURSOR"
    // mapped from the `$orderby` field — see modkit-odata problem mapping).
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let s = String::from_utf8_lossy(&body);
    assert!(s.contains("orderby") || s.contains("cursor"));
}

#[tokio::test]
async fn cursor_only_is_ok() {
    async fn handler(OData(_q): OData) -> &'static str {
        "ok"
    }

    let app = Router::new().route("/", get(handler));

    // Provide only cursor
    let req = Request::builder()
        .uri("/?cursor=eyJ2IjoxLCJrIjpbIjEiXS")
        .body(axum::body::Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // Expect a 400 for the malformed cursor, but the body must NOT mention
    // an orderby/cursor conflict (this request only carries `cursor`).
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let s = String::from_utf8_lossy(&body);
    assert!(!s.contains("orderby") || !s.contains("both"));
}

#[tokio::test]
async fn orderby_only_is_ok() {
    async fn handler(OData(_q): OData) -> &'static str {
        "ok"
    }

    let app = Router::new().route("/", get(handler));

    // Provide only $orderby
    let req = Request::builder()
        .uri("/?$orderby=id%20desc")
        .body(axum::body::Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
