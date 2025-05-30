use axum::{
    http::StatusCode,
    extract::Form,
    response::{Html, IntoResponse, Response},
    routing::{get, get_service},
    Router,
};
use askama::Template;
use serde::{Deserialize, Serialize};
use std::{fs, io, net::SocketAddr};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TapReadings {
    tap_one: HomeBrew,
    tap_two: HomeBrew,
    tap_three: HomeBrew,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HomeBrew {
    name: String,
    state: f64
}

pub struct Tap {
    name: String,
    state: f64,
    volume_left: f64
}

pub fn get_readings() -> TapReadings {
    let json = &fs::File::open("./data/tap.json").unwrap();
    let tap_readings: TapReadings = serde_json::from_reader(json).unwrap();
    tap_readings
}

#[derive(Template)]
#[template(path = "index.html")]
struct KegeratorDisplatTemplate {
    tap_one: Tap,
    tap_two: Tap,
    tap_three: Tap,
}
struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}


pub fn format_tap(homebrew:HomeBrew) -> Tap {
    Tap {name: homebrew.name, state: ((homebrew.state/ 19f64) * 100f64).round(), volume_left: homebrew.state }
}


#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "example_templates=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // build our application with some routes
    let app = Router::new().route("/", get(keg_display))
        .route("/form", get(show_form).post(accept_form))
        .route("/static", get(|| async { "Hi from /static" }))
        .fallback(get_service(ServeDir::new(".")).handle_error(handle_error))
        .layer(TraceLayer::new_for_http());

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn keg_display() -> impl IntoResponse {
    let tap_readings = get_readings();
    let template = KegeratorDisplatTemplate {
                 tap_one:  format_tap(tap_readings.tap_one),
                 tap_two: format_tap(tap_readings.tap_two),
                 tap_three: format_tap(tap_readings.tap_three),
        
             };
    HtmlTemplate(template)
}

async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

async fn show_form() -> Html<&'static str> {
    Html(
        r#"
        <!doctype html>
        <html>
            <head></head>
            <body>
                <form action="/form" method="post">
                    <label for="name">
                        Enter your name:
                        <input type="text" name="name">
                    </label>
                    <label>
                        Enter your email:
                        <input type="text" name="email">
                    </label>
                    <input type="submit" value="Subscribe!">
                </form>
            </body>
        </html>
        "#,
    )
}

#[derive(Deserialize, Debug)]
struct Input {
    name: String,
    email: String,
}

async fn accept_form(Form(input): Form<Input>) {
    print!("name: {}", &input.name);
    print!("email: {}", &input.email);
}