use std::path::PathBuf;

use rocket::Config;
use rocket_contrib::{serve::StaticFiles, templates::Template};

mod cgi;
mod db;
mod guards;
mod routes;
mod util;

use routes::vcs::git::http_backend::GitHttpBackend;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let config = Config::default();

    let data_dir = PathBuf::from(util::ensure_correct_path_separator(
        util::read_expected_env_var("SOURCESHACK_DATA_DIR"),
    ));

    rocket::custom(config.clone())
        .manage(config)
        .mount("/", routes::front_page::routes())
        .mount("/", routes::account::routes())
        .mount("/", routes::vcs::git::web::routes())
        .mount("/", GitHttpBackend::new(data_dir.join("git_repos")))
        .mount("/static", StaticFiles::from("static").rank(-100))
        .attach(Template::fairing())
        .attach(db::Postgres::fairing())
        .launch()
        .await
        .unwrap();
}
