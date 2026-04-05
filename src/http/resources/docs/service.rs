use axum::{Json, response::Response};
use once_cell::sync::Lazy;
use utoipa::OpenApi;

use crate::http::resources::views::service as views;

#[derive(OpenApi)]
#[openapi(
    paths(
        views::get_all_views,
        views::get_view,
        views::create_view,
        views::update_view,
        views::delete_view,
        views::get_entry_point_view,
        views::set_entry_point_view
    ),
    components(
        schemas(
            crate::http::resources::views::service::View,
            crate::http::resources::views::service::ViewPage,
            crate::http::resources::views::service::ViewInput,
            crate::http::ApiResponseView,
            crate::http::ApiResponseViewList,
            crate::http::ApiResponseViewPage,
            crate::http::ApiResponseEmpty
        )
    ),
    tags(
        (name = "Views", description = "Canvas views CRUD")
    )
)]
pub struct ApiDoc;

pub static OPENAPI: Lazy<utoipa::openapi::OpenApi> = Lazy::new(|| ApiDoc::openapi());

pub async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(OPENAPI.clone())
}

pub async fn swagger_ui() -> Response {
    const HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
  <title>LiRAYS-SCADA API</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.onload = () => {
      SwaggerUIBundle({
        url: '/api-docs/openapi.json',
        dom_id: '#swagger-ui'
      });
    };
  </script>
</body>
</html>"#;
    Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(axum::body::Body::from(HTML))
        .unwrap()
}
