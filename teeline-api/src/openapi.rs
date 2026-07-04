use axum::Router;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa_scalar::{Scalar, Servable};

use crate::{
    AppState,
    models::{request, response},
    routes,
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .as_mut()
            .expect("components are registered via #[openapi(components(...))]");
        components.add_security_scheme(
            "bearer_token",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
        );
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Api-Key"))),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::index::handler,
        routes::health::handler,
        routes::solvers::list_solvers,
        routes::parse::parse,
        routes::solve::solve,
    ),
    components(schemas(
        routes::index::IndexResponse,
        routes::health::HealthResponse,
        request::HeuristicConfig,
        request::NnConfig,
        request::TwoOptConfig,
        request::ThreeOptConfig,
        request::OrOptConfig,
        request::TabuConfig,
        request::StochasticHillConfig,
        request::PsoConfig,
        request::GsaConfig,
        request::LkConfig,
        request::SaConfig,
        request::GaConfig,
        request::CsConfig,
        request::FpaConfig,
        request::SomConfig,
        request::FourierConfig,
        request::SolverConfigs,
        request::CityInput,
        request::TspInput,
        request::ParseRequest,
        request::SolveRequest,
        response::AlgorithmInfo,
        response::CityDto,
        response::ParseResponse,
        response::SolveResponse,
    )),
    tags(
        (name = "tsp", description = "Traveling Salesman Problem solver endpoints")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

pub fn openapi_router() -> Router<AppState> {
    let openapi = ApiDoc::openapi();
    Router::new()
        .route(
            "/openapi.json",
            axum::routing::get({
                let openapi = openapi.clone();
                move || async move { axum::Json(openapi) }
            }),
        )
        .merge(Scalar::with_url("/docs", openapi))
}
