use axum::Router;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::{
    AppState,
    models::{request, response},
    routes,
};

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
    )
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
