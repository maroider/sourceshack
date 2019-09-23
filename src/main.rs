#![feature(proc_macro_hygiene, decl_macro)]

use rocket::Config;
use rocket_contrib::templates::Template;

mod rocket_cgi;
mod routes;
mod util;
mod vcs;

use vcs::git::GitHttpBackend;

fn main() {
    dotenv::dotenv().ok();

    let config = Config::active().unwrap();
    rocket::custom(config.clone())
        .manage(config)
        .mount("/", routes::vcs::git::routes())
        .mount("/", GitHttpBackend::new())
        .attach(Template::fairing())
        .launch();
}
