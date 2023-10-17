use std::path::{PathBuf, Path};
use actix_files as afs;
use actix_web::{get, App, HttpRequest, HttpServer, Responder, HttpResponse, http::header::ContentType, middleware};
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tera::{ Tera, Context };
use pulldown_cmark::{ Parser, Options, html::push_html };
use log::{ info, warn, error };
use tokio::sync::RwLock;
use tokio::fs;
use futures::{SinkExt, channel::mpsc::{channel, Receiver}};
use notify::{RecommendedWatcher, Event, Config, Watcher, RecursiveMode, EventKind};

mod conditional;
use conditional::Conditional;

const BLOG_CONFIG: &str = "./Blogs.toml";

static TEMPLATES: Lazy<Tera> = Lazy::new(|| Tera::new("templates/*.tera").expect("Failed to parse templates"));

static BLOGS: Lazy<RwLock<BlogConfig>> = Lazy::new(|| {
    let blogs = std::fs::read_to_string(BLOG_CONFIG).expect("Unable to read `Blogs.toml`");
    RwLock::new(toml::from_str(&blogs).expect("Unable to parse `blogs/blogs.json`"))
});

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    tokio::spawn(watch_blogs());

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

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BlogConfig {
    output: PathBuf,
    input: PathBuf,
    tables: bool,
    footnotes: bool,
    strikethrough: bool,
    tasklists: bool,
    smart_punctuation: bool,
    heading_attributes: bool,
    blogs: Vec<BlogPost>,
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    let watcher = notify::recommended_watcher(
        move |res| {
            tokio::runtime::Handle::current().block_on(async {
                match tx.send(res).await {
                    Ok(_) => {},
                    Err(err) => error!("Failed to send file event through channel: {err}"),
                }
            });
        },
    )?;

    Ok((watcher, rx))
}

async fn watch_blogs() {
    let (mut watcher, mut rx) = match async_watcher() {
        Ok(ok) => ok,
        Err(err) => return error!("Unable to create async watcher: {err}"),
    };

    if let Err(err) = watcher.watch(BLOG_CONFIG.as_ref(), RecursiveMode::NonRecursive) {
        return error!("Unable to watch `{BLOG_CONFIG}`: {err}");
    }
    if let Err(err) = watcher.watch(BLOGS.read().await.input.as_ref(), RecursiveMode::Recursive) {
        return error!("Unable to watch blog input directory: {err}");
    };

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => {
                handle_event(&mut watcher, event).await;
            },
            Err(err) => error!("error while watching files: {err}")
        }
    }
}

async fn handle_event(watcher: &mut RecommendedWatcher, event: Event) -> anyhow::Result<()> {
    if matches!(event.kind, EventKind::Any | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Other) {
        if let Some(BLOG_CONFIG) = event.source() {
            watcher.unwatch(BLOGS.read().await.input.as_ref())?;
            *BLOGS.write().await = toml::from_str(&fs::read_to_string(BLOG_CONFIG).await?)?;
            watcher.watch(BLOGS.read().await.input.as_ref(), RecursiveMode::Recursive)?;
        }

        render_md().await?;
    }

    Ok(())
}

/// Render and sanitize blog markdown
async fn render_md() -> anyhow::Result<()> {
    for blog in BLOGS.read().await.blogs.iter() {
        let rendered = {
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
            let md = fs::read_to_string(md_path).await?;

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
            TEMPLATES.render("blog.tera", &ctx).expect("Unable to render blog")
        };
        fs::write(&blog.out, rendered).await?;
    }

    Ok(())
}

#[get("/")]
async fn index(_req: HttpRequest) -> impl Responder {
    let render = Context::from_serialize(&*BLOGS.read().await).map(|ctx| {
        TEMPLATES.render("index.tera", &ctx)
    });

    match render {
        Ok(Ok(html)) => HttpResponse::Ok().content_type(ContentType::html()).body(html),
        Err(err) | Ok(Err(err)) => HttpResponse::InternalServerError().body(err.to_string()),
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
