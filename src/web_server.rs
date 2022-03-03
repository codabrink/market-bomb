use crate::prelude::*;
use actix_web::{
  middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use qstring::QString;
use std::{env, io, io::Error};

async fn candles(req: HttpRequest) -> impl Responder {
  // let qs = QString::from(req.query_string());
  "hello"
}

#[actix_web::main]
pub async fn run() -> io::Result<()> {
  env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
  env_logger::init();

  HttpServer::new(move || {
    App::new()
      .wrap(middleware::Logger::default())
      .route("/candles", web::get().to(candles))
  })
  .bind("0.0.0.0:8080")?
  .run()
  .await
}
