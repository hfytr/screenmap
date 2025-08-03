#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::body::Body;
    use axum::extract::{Path, RawQuery, State};
    use axum::http::HeaderMap;
    use axum::response::{IntoResponse, Response};
    use axum::{Router, http::Request, routing::get};
    use leptos::prelude::{get_configuration, provide_context};
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use log::info;
    use screenmap::{
        app::{App, shell},
        server::AppState,
    };
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;

    async fn server_fn_handler(
        State(state): State<AppState>,
        path: Path<String>,
        _: HeaderMap,
        _: RawQuery,
        request: Request<Body>,
    ) -> impl IntoResponse {
        info!("REQUEST {:?}", path);
        leptos_axum::handle_server_fns_with_context(move || provide_context(state.clone()), request)
            .await
    }

    async fn leptos_routes_handler(State(state): State<AppState>, req: Request<Body>) -> Response {
        let new_state = state.clone();
        let handler = leptos_axum::render_app_to_stream_with_context(
            move || provide_context(new_state.clone()),
            move || shell(state.leptos_options.clone()),
        );
        handler(req).await.into_response()
    }

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let postgres_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is unset.");
    let pool = Arc::new(PgPoolOptions::new().connect(&postgres_url).await.unwrap());
    let state = AppState {
        pool,
        leptos_options,
    };
    let routes = generate_route_list(App);

    let app: Router = Router::new()
        .route(
            "/api/{*fn_name}",
            get(server_fn_handler).post(server_fn_handler),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .with_state(state);

    info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
