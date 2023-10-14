use std::fs;
use std::path::PathBuf;
use actix_files as afs;
use actix_web::{get, App, HttpRequest, HttpServer, Responder, HttpResponse, http::header::ContentType};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tera::{ Tera, Context };
use pulldown_cmark::{ Parser, Options, html::push_html };

static TEMPLATES: Lazy<Tera> = Lazy::new(|| Tera::new("templates/*.tera").expect("Failed to parse templates"));

static BLOGS: Lazy<Vec<BlogPost>> = Lazy::new(|| {
    serde_json::from_str(include_str!("../blogs/blogs.json")).expect("Unable to parse `blogs/blogs.json`")
});

static INDEX_CTX: Lazy<Context> = Lazy::new(|| {
    Context::from_serialize(IndexCtx::from(BLOGS.clone())).expect("Unable to create `tera::Context` from `BlogPost`")
});

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Render and sanitize blog markdown
    for blog in BLOGS.iter() {
        let mut ctx = Context::new();
        ctx.insert("title", &blog.title);
        ctx.insert("thumbnail", &blog.thumbnail);

        let md_path = PathBuf::from("./blogs").join(&blog.file);
        let md = fs::read_to_string(md_path)?;

        let options = Options::ENABLE_TABLES;
        let md_parse = Parser::new_ext(&md, options);

        let mut unsafe_html = String::new();
        push_html(&mut unsafe_html, md_parse);

        let safe_html = ammonia::clean(&unsafe_html);
        ctx.insert("body", &safe_html);

        println!("Rendered `{}`, output at `{}`", blog.file.display(), blog.out.display());
        fs::write(&blog.out, TEMPLATES.render("blog.tera", &ctx).expect("Unable to render blog"))?;
    }

    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(favicon)
            .service(robots)
            .service(afs::Files::new("/static", "./static").show_files_listing())
            .service(afs::Files::new("/js", "./js").show_files_listing())
            .service(afs::Files::new("/css", "./css").show_files_listing())
            .service(afs::Files::new("/blog", "./blogs/out"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BlogPost {
    title: String,
    description: String,
    file: PathBuf,
    out: PathBuf,
    href: String,
    thumbnail: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct IndexCtx {
    blogs: Vec<BlogPost>,
}

impl From<Vec<BlogPost>> for IndexCtx {
    fn from(value: Vec<BlogPost>) -> Self {
        IndexCtx {blogs: value}
    }
}

#[get("/")]
async fn index(_req: HttpRequest) -> impl Responder {
    match TEMPLATES.render("index.tera", &INDEX_CTX) {
        Ok(html) => HttpResponse::Ok().content_type(ContentType::html()).body(html),
        Err(err) => HttpResponse::InternalServerError().body(format!("{err}")),
    }
}

#[get("/favicon.ico")]
async fn favicon(_req: HttpRequest) -> impl Responder {
    afs::NamedFile::open("static/favicon.ico")
}

#[get("/robots.txt")]
async fn robots(_req: HttpRequest) -> impl Responder {
    afs::NamedFile::open("static/robots.txt")
}
