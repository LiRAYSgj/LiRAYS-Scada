use std::sync::LazyLock;

use axum::{Json, response::Response};
use log::warn;
use utoipa::OpenApi;

use crate::http::resources::inbound::service as inbound;
use crate::http::resources::outbound::service as outbound;
use crate::http::resources::plugin_collections;
use crate::http::resources::plugin_instances;
use crate::http::resources::views::service as views;

#[derive(OpenApi)]
#[openapi(
    paths(
        inbound::plugins, inbound::list, inbound::get, inbound::create, inbound::update, inbound::delete,
        outbound::plugins, outbound::list, outbound::get, outbound::create, outbound::update, outbound::delete,
        plugin_collections::list,
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
            views::ViewSortBy, views::SortDirection,
            crate::http::ApiResponseView,
            crate::http::ApiResponseViewPage,
            plugin_instances::InstanceInput, plugin_instances::InstanceUpdate, plugin_instances::InstanceInfo, plugin_instances::InstanceStatus, plugin_instances::InstanceState,
            plugin_instances::PluginCollection, crate::plugin_form::PluginDescriptor,
            crate::http::ApiResponseEmpty
        )
    ),
    modifiers(&Security),
    tags(
        (name = "Inbound", description = "Admin-only inbound connections and compiled plugins"),
        (name = "Outbound", description = "Admin-only outbound connections and compiled plugins"),
        (name = "Plugin collections", description = "Admin-only instance collections and plugin configuration forms"),
        (name = "Views", description = "Canvas views CRUD")
    )
)]
pub struct ApiDoc;

struct Security;

impl utoipa::Modify for Security {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}

pub static OPENAPI: LazyLock<utoipa::openapi::OpenApi> = LazyLock::new(|| ApiDoc::openapi());

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
        .unwrap_or_else(|e| {
            warn!("Failed building swagger UI response: {e}");
            Response::new(axum::body::Body::from("Swagger UI unavailable"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_references_resolve_and_include_plugin_endpoints() {
        let document = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let mut pending = vec![&document];
        while let Some(value) = pending.pop() {
            match value {
                serde_json::Value::Object(fields) => {
                    if let Some(reference) = fields.get("$ref").and_then(|v| v.as_str()) {
                        if let Some(pointer) = reference.strip_prefix('#') {
                            assert!(
                                document.pointer(pointer).is_some(),
                                "Unresolved schema: {reference}"
                            );
                        }
                    }
                    pending.extend(fields.values());
                }
                serde_json::Value::Array(values) => pending.extend(values),
                _ => {}
            }
        }
        // Swagger UI routes "Try it out" by operationId, so a duplicate sends the
        // request to another endpoint.
        let mut operation_ids = std::collections::HashSet::new();
        for path in document["paths"].as_object().unwrap().values() {
            for operation in path.as_object().unwrap().values() {
                let id = operation["operationId"].as_str().unwrap();
                assert!(operation_ids.insert(id), "Duplicate operationId: {id}");
            }
        }
        assert!(document["paths"]["/api/inbound"]["post"].is_object());
        assert!(document["paths"]["/api/outbound"]["post"].is_object());
        assert!(document["paths"]["/api/plugin-collections"]["get"].is_object());
        assert!(document["components"]["securitySchemes"]["bearerAuth"].is_object());
    }
}
