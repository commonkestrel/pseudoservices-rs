use std::fs;
use std::path::PathBuf;
use actix_files as afs;
use actix_web::{get, App, HttpRequest, HttpServer, Responder, HttpResponse, http::header::ContentType, middleware};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tera::{ Tera, Context };
use pulldown_cmark::{ Parser, Options, html::push_html };
use log::info;

mod conditional;
use conditional::Conditional;

static TEMPLATES: Lazy<Tera> = Lazy::new(|| Tera::new("templates/*.tera").expect("Failed to parse templates"));

static BLOGS: Lazy<Vec<BlogPost>> = Lazy::new(|| {
    let blogs = fs::read_to_string("blogs/blogs.json").expect("Unable to read `blogs/blogs.json`");
    serde_json::from_str(&blogs).expect("Unable to parse `blogs/blogs.json`")
});

static INDEX_CTX: Lazy<Context> = Lazy::new(|| {
    Context::from_serialize(IndexCtx::from(BLOGS.clone())).expect("Unable to create `tera::Context` from `BlogPost`")
});

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Render and sanitize blog markdown
    for blog in BLOGS.iter() {
        let mut ctx = Context::new();
        ctx.insert("title", &blog.title);
        ctx.insert("description", &blog.description);
        ctx.insert("thumbnail", &blog.thumbnail);
        if let Some(next) = &blog.next {
            ctx.insert("next", next);
        }
        if let Some(prev) = &blog.prev {
            ctx.insert("prev", prev);
        }

        let md_path = PathBuf::from("./blogs").join(&blog.file);
        let md = fs::read_to_string(md_path)?;

        let options = Options::all();
        let md_parse = Parser::new_ext(&md, options);

        let mut unsafe_html = String::new();
        push_html(&mut unsafe_html, md_parse.into_iter());

        let id = ["id"];

        let safe_html = ammonia::Builder::new()
            .add_tag_attributes("code", &["class"])
            .add_tag_attributes("h1", &id)
            .add_tag_attributes("h2", &id)
            .add_tag_attributes("h3", &id)
            .add_tag_attributes("h4", &id)
            .add_tag_attributes("h5", &id)

            .clean(&unsafe_html)
            .to_string();

        ctx.insert("body", &safe_html);

        info!("Rendered `{}`, output at `{}`", blog.file.display(), blog.out.display());
        fs::write(&blog.out, TEMPLATES.render("blog.tera", &ctx).expect("Unable to render blog"))?;
    }


    let mut server = HttpServer::new(|| {
        let nocache = middleware::DefaultHeaders::new()
            .add(("Cache-Control", "no-cache, no-store, must-revalidate"))
            .add(("Pragma", "no-cache"))
            .add(("Expires", 0));

        App::new()
            .service(index)
            .service(favicon)
            .service(robots)
            .service(afs::Files::new("/static", "./static").show_files_listing())
            .service(afs::Files::new("/js", "./js").show_files_listing())
            .service(afs::Files::new("/css", "./css").show_files_listing())
            .service(afs::Files::new("/blog", "./blogs/out"))
            .service(afs::Files::new("/host", "./host"))
            .wrap(Conditional::new(nocache, cfg!(debug_assertions)))
            .wrap(middleware::Logger::default())
    });

    // Allow for hosting config based on environment variables
    match std::env::var("PS_ADDRS") {
        Ok(addrs) => {
            for addr in addrs.split(' ') {
                let (ip, port) = addr.split_once(':').expect(&format!("Unable to parse ip address: {addr}"));
                server = server.bind((ip, port.parse().expect(&format!("Unable to parse port {port}"))))?
            }
        },
        Err(_) => {
            server = server.bind(("localhost", 80))?;
        }
    };
    
    server.run().await
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BlogPost {
    title: String,
    description: String,
    file: PathBuf,
    out: PathBuf,
    href: String,
    card: String,
    thumbnail: String,
    next: Option<Linked>,
    prev: Option<Linked>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Linked {
    title: String,
    href: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct IndexCtx {
    blogs: Vec<BlogPost>,
}

impl From<Vec<BlogPost>> for IndexCtx {
    fn from(mut value: Vec<BlogPost>) -> Self {
        value.reverse();
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
