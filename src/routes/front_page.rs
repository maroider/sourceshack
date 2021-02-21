use rocket::{get, routes, Route};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

pub fn routes() -> Vec<Route> {
    routes![front_page]
}

#[get("/")]
pub fn front_page() -> Template {
    Template::render("front_page", FrontPageContext { users: vec![] })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FrontPageContext {
    users: Vec<User>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct User {
    name: String,
}
