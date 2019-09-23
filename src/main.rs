#![feature(proc_macro_hygiene, decl_macro)]

use rocket::{fairing::AdHoc, routes, Config};
use rocket_contrib::templates::Template;

mod rocket_cgi;
mod vcs;

use vcs::git::GitHttpBackend;

fn main() {
    dotenv::dotenv().ok();

    let config = Config::active().unwrap();
    rocket::custom(config.clone())
        .manage(config)
        .mount("/", routes![])
        .mount("/", GitHttpBackend::new())
        .attach(Template::fairing())
        .attach(AdHoc::on_request("print_request", |req, _data| {
            println!("{:#?}", req)
        }))
        .launch();
}
