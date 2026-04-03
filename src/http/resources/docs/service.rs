use axum::{
    response::Response,
    Json,
};
use once_cell::sync::Lazy;
use utoipa::OpenApi;

use crate::http::resources::resources::service as res;

#[derive(OpenApi)]
#[openapi(
    paths(
        res::get_all_resources,
        res::get_resource,
        res::create_resource,
        res::update_resource,
        res::delete_resource
    ),
    components(
        schemas(
            crate::http::resources::resource::service::StaticResource,
            crate::http::resources::resource::service::StaticResourceInput,
            crate::http::ApiResponseResource,
            crate::http::ApiResponseResourceList,
            crate::http::ApiResponseEmpty
        )
    ),
    tags(
        (name = "Resources", description = "Static resources CRUD")
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
