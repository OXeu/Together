use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use actix::*;
use actix_files::{Files, NamedFile};
use actix_web::{
    middleware::Logger, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;

mod context;
mod server;
mod session;

async fn index() -> impl Responder {
    let static_path = std::env::var("STATIC").unwrap_or("./static".to_owned());
    NamedFile::open_async(static_path + "/index.html")
        .await
        .unwrap()
}

/// Entry point for our websocket route
async fn chat_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::ChatServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        session::WsChatSession {
            id: "".to_string(),
            hb: Instant::now(),
            room: "".to_owned(), //Empty Room
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

/// Displays state
async fn get_count(count: web::Data<AtomicUsize>) -> impl Responder {
    let current_count = count.load(Ordering::SeqCst);
    format!("Visitors: {current_count}")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // set up applications state
    // keep a count of the number of visitors
    let app_state = Arc::new(AtomicUsize::new(0));

    // start chat server actor
    let server = server::ChatServer::new(app_state.clone()).start();

    log::info!("starting HTTP server at http://127.0.0.1:8000");
    let port = std::env::var("PORT").unwrap_or("8000".to_owned()).parse::<u16>().unwrap();
    HttpServer::new(move || {
        let static_path = std::env::var("STATIC").unwrap_or("./static".to_owned());
        App::new()
            .app_data(web::Data::from(app_state.clone()))
            .app_data(web::Data::new(server.clone()))
            .service(web::resource("/").to(index))
            .route("/count", web::get().to(get_count))
            .route("/ws", web::get().to(chat_route))
            .service(Files::new("/", static_path))
            .wrap(Logger::default())
    })
    .workers(6)
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
