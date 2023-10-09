use actix_files as fs;
use actix_web::{get, App, HttpServer, HttpRequest, Responder};

#[get("/")]
async fn index(req: HttpRequest) -> impl Responder {
    fs::NamedFile::open("html/index.html")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(fs::Files::new("/static", "./static").show_files_listing())
            .service(fs::Files::new("/js", "./js").show_files_listing())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}