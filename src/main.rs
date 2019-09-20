#![feature(proc_macro_hygiene, decl_macro)]

use rocket::routes;
use rocket_contrib::templates::Template;

fn main() {
    dotenv::dotenv().ok();

    rocket::ignite()
        .mount("/", routes![])
        .attach(Template::fairing())
        .launch();
}
