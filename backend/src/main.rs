use std::{path::PathBuf, time::Duration};
use actix_files as afs;
use afs::NamedFile;
use actix_web::{get, error, App, HttpRequest, HttpServer, Responder, HttpResponse, http::header::ContentType, middleware};
use anyhow::bail;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tera::{ Tera, Context };
use pulldown_cmark::{ Parser, Options, html::push_html };
use log::{ info, error };
use tokio::{sync::{RwLock, mpsc::{channel, Receiver}}, fs, runtime::Handle};
use  notify_debouncer_full::{ new_debouncer, notify::{self, RecommendedWatcher, Watcher, RecursiveMode, EventKind}, DebouncedEvent, FileIdMap, Debouncer, DebounceEventResult };
use path_clean::PathClean;

mod conditional;
use conditional::Conditional;

const BLOG_CONFIG: &str = "Blogs.toml";

static TEMPLATES: Lazy<Tera> = Lazy::new(|| Tera::new("templates/*.tera").expect("Failed to parse templates"));

static BLOGS: Lazy<RwLock<BlogConfig>> = Lazy::new(|| {
    let blogs = std::fs::read_to_string(BLOG_CONFIG).expect("Unable to read `Blogs.toml`");
    RwLock::new(toml::from_str(&blogs).expect("Unable to parse `blogs/blogs.json`"))
});

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    if let Err(err) = render_md().await {
        error!("error rendering markdown: {err}");
    };
    
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
            .service(blog_post)
            .service(afs::Files::new("/static", "./static").show_files_listing())
            .service(afs::Files::new("/js", "./js").show_files_listing())
            .service(afs::Files::new("/css", "./css").show_files_listing())
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
    infile: PathBuf,
    outfile: PathBuf,
    card: String,
    thumbnail: String,
    next: Option<Linked>,
    prev: Option<Linked>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MdConfig {
    #[serde(default = "_default_true")]
    tables: bool,
    #[serde(default = "_default_true")]
    footnotes: bool,
    #[serde(default = "_default_true")]
    strikethrough: bool,
    #[serde(default = "_default_true")]
    tasklists: bool,
    #[serde(default = "_default_true")]
    smart_punctuation: bool,
    #[serde(default = "_default_true")]
    heading_attributes: bool,
}

const fn _default_true() -> bool { true }

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Linked {
    title: String,
    href: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct IOConfig {
    output: PathBuf,
    input: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BlogConfig {
    config: IOConfig,
    markdown: MdConfig,
    articles: Vec<BlogPost>,
}

fn async_watcher() -> notify::Result<(Debouncer<RecommendedWatcher, FileIdMap>, Receiver<DebouncedEvent>)> {
    let (tx, rx) = channel(1);
    let rt = Handle::current();

    let debouncer = new_debouncer(Duration::from_millis(5000), None, move |res: DebounceEventResult| {
        let tx = tx.clone();

        rt.spawn(async move {
            match res {
                Ok(events) => if let Some(event) = events.last() {
                    if let Err(err) = tx.send(event.clone()).await {
                        error!("Error sending notify event: {err}");
                    }
                },
                Err(err) => error!("Error in notify watcher: {err:?}"),
            }
        });
    })?;

    Ok((debouncer, rx))
}

async fn watch_blogs() {
    let (mut watcher, mut rx) = match async_watcher() {
        Ok(ok) => ok,
        Err(err) => return error!("Unable to create async watcher: {err}"),
    };

    if let Err(err) = watcher.watcher().watch(BLOG_CONFIG.as_ref(), RecursiveMode::NonRecursive) {
        return error!("Unable to watch `{BLOG_CONFIG}`: {err}");
    }
    if let Err(err) = watcher.watcher().watch(BLOGS.read().await.config.input.as_ref(), RecursiveMode::NonRecursive) {
        return error!("Unable to watch blog input directory: {err}");
    };

    while let Some(event) = rx.recv().await {
        info!("{event:#?}");
        if let Err(err) = handle_event(watcher.watcher(), event).await {
            error!("Error handling file change event: {err}");
        };
    }
}

async fn handle_event(watcher: &mut RecommendedWatcher, event: DebouncedEvent) -> anyhow::Result<()> {
    if !matches!(event.kind, EventKind::Remove(_) | EventKind::Access(_)) || event.need_rescan() {
        if let Some(Some(Some(BLOG_CONFIG))) = event.paths.first().map(|path| path.file_name().map(|name| name.to_str())) {
            watcher.unwatch(BLOGS.read().await.config.input.as_ref())?;
            match toml::from_str(&fs::read_to_string(BLOG_CONFIG).await?) {
                Ok(cfg) => {
                    info!("success");
                    *BLOGS.write().await = cfg;
                },
                Err(err) => bail!("Unable to parse config file: {err}"),
            }
            watcher.watch(BLOGS.read().await.config.input.as_ref(), RecursiveMode::Recursive)?;
        }

        render_md().await?;
    }

    Ok(())
}

/// Render and sanitize blog markdown
async fn render_md() -> anyhow::Result<()> {
    let blogs = BLOGS.read().await;
    for blog in blogs.articles.iter() {
        let output = blogs.config.output.join(&blog.outfile);
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

            let md_path = blogs.config.input.join(&blog.infile);
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

            info!("Rendered `{}`, output at `{}`", blog.infile.display(), output.display());
            TEMPLATES.render("blog.tera", &ctx).expect("Unable to render blog")
        };
        fs::write(output, rendered).await?;
    }

    Ok(())
}

#[get("/")]
async fn index(_req: HttpRequest) -> impl Responder {
    let blogs = BLOGS.read().await;
    let render = Context::from_serialize(&*blogs).map(|ctx| {
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

#[get("/blog/{filename}")]
async fn blog_post(req: HttpRequest) -> Result<NamedFile, error::Error> {
    let filename: PathBuf = req.match_info().query("filename").parse()?;
    
    let file_path = BLOGS
        .read()
        .await
        .config
        .output
        .join(
            match filename.clean().file_name() {
                Some(f) => f,
                None => Err(error::ErrorNotFound("File not found"))?,
            }
        );

    let file = afs::NamedFile::open(&file_path).map_err(|err| error::ErrorNotFound(err))?;

    Ok(file)
}
