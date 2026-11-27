use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::http::resources::views::service::{View, ViewPage};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApiResponseView {
    success: bool,
    data: Option<View>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApiResponseViewPage {
    success: bool,
    data: Option<ViewPage>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApiResponseEmpty {
    success: bool,
    data: Option<()>,
    message: Option<String>,
}

impl ApiResponseEmpty {
    pub(crate) fn success() -> Self {
        Self {
            success: true,
            data: Some(()),
            message: None,
        }
    }

    pub(crate) fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

impl<T> ApiResponse<T> {
    pub(crate) fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }
    }
}
