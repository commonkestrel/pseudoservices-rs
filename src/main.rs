use actix_files as fs;
use actix_web::{get, App, HttpServer, HttpRequest, Responder};
use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;

lazy_static! {
    static ref BLOGS: Vec<BlogPost> = serde_json::from_str(include_str!("blogs.json")).expect("unable to deserialize `blogs.json`");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(favicon)
            .service(robots)
            .service(fs::Files::new("/static", "./static").show_files_listing())
            .service(fs::Files::new("/js", "./js").show_files_listing())
            .service(fs::Files::new("/css", "./css").show_files_listing())
            .service(fs::Files::new("/fonts", "./fonts").show_files_listing())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[derive(Deserialize)]
struct BlogPost {
    title: String,
    description: String,
    href: String,
    thumbnail: String,
}

#[get("/")]
async fn index(_req: HttpRequest) -> impl Responder {
    fs::NamedFile::open("html/index.html")
}

#[get("/favicon.ico")]
async fn favicon(_req: HttpRequest) -> impl Responder {
    fs::NamedFile::open("static/favicon.ico")
}

#[get("/robots.txt")]
async fn robots(_req: HttpRequest) -> impl Responder {
    fs::NamedFile::open("static/robots.txt")
}
