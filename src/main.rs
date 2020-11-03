#![feature(proc_macro_hygiene, decl_macro)]

use std::path::PathBuf;

use rocket::Config;
use rocket_contrib::{serve::StaticFiles, templates::Template};

mod cgi;
mod routes;
mod util;

use routes::vcs::git::http_backend::GitHttpBackend;

fn main() {
    dotenv::dotenv().ok();

    let config = Config::active().unwrap();

    let data_dir = PathBuf::from(util::ensure_correct_path_separator(
        util::read_expected_env_var("SOURCESHACK_DATA_DIR"),
    ));

    rocket::custom(config.clone())
        .manage(config)
        .mount("/", routes::vcs::git::web::routes())
        .mount("/", GitHttpBackend::new(data_dir.join("git_repos")))
        .mount("/static", StaticFiles::from("static").rank(-100))
        .attach(Template::fairing())
        .launch();
}
