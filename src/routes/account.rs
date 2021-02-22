use password_hash::{HasherError, PasswordHash, PasswordHasher, Salt, SaltString};
use pbkdf2::Pbkdf2;
use rand_core::OsRng;
use rocket::{
    get, post,
    request::{Form, FromForm},
    routes, Route,
};
use rocket_contrib::templates::Template;

use crate::{db::Postgres, util::tera_dummy_ctx};

pub fn routes() -> Vec<Route> {
    routes![sign_up, do_sign_up, sign_in, do_sign_in]
}

#[get("/sign-up")]
fn sign_up() -> Template {
    Template::render("sign_up", tera_dummy_ctx())
}

#[post("/sign-up", data = "<form>")]
async fn do_sign_up<'r>(pg: Postgres<'r>, form: Form<SignUp>) -> Result<String, String> {
    let rng = OsRng::default();
    let salt = SaltString::generate(rng);
    let hash = hash_password(form.password.as_bytes(), salt.as_salt())
        .map_err(|err| format!("{:#?}", err))?;
    let emails: &[String] = &[form.email.clone()];
    sqlx::query!(
        r#"
        INSERT INTO public.users
            (userid, username, emails, password_hash)
        VALUES
            (gen_random_uuid(), $1, $2, $3)"#,
        form.username,
        emails,
        format!("{}", hash),
    )
    .execute(pg)
    .await
    .map_err(|err| format!("{:#?}", err))?;
    Ok(format!("registration complete"))
}

#[derive(Debug, FromForm)]
struct SignUp {
    username: String,
    email: String,
    password: String,
}

#[get("/sign-in")]
fn sign_in() -> Template {
    Template::render("sign_in", tera_dummy_ctx())
}

#[post("/sign-in", data = "<form>")]
async fn do_sign_in<'r>(pg: Postgres<'r>, form: Form<SignIn>) -> Result<String, String> {
    let username = form.login.clone();
    let emails: &[String] = &[form.login.clone()];
    let query_result = sqlx::query!(
        r#"
        SELECT
            userid, password_hash
        FROM (
            SELECT
                userid, username, emails, password_hash
            FROM
                public.users
            WHERE
                username = $1 OR $2 IN(emails)
        ) as subquery"#,
        username,
        emails,
    )
    .fetch_optional(pg)
    .await
    .map_err(|err| format!("{:#?}", err))?;
    if let Some(user) = query_result {
        let password_hash =
            PasswordHash::new(&user.password_hash).map_err(|err| format!("{:#?}", err))?;
        let login_hash = hash_password(form.password.as_bytes(), password_hash.salt.unwrap())
            .map_err(|err| format!("{:#?}", err))?;
        if login_hash.hash == password_hash.hash {
            Ok(format!("Login successful"))
        } else {
            Err(format!("Invalid password"))
        }
    } else {
        Err(format!("No such username or email adress extists"))
    }
}

#[derive(Debug, FromForm)]
struct SignIn {
    login: String,
    password: String,
}

fn hash_password<'a>(password: &[u8], salt: Salt<'a>) -> Result<PasswordHash<'a>, HasherError> {
    Pbkdf2.hash_password(
        password,
        None,
        None,
        pbkdf2::Params {
            rounds: 10_000,
            output_length: 32,
        },
        salt,
    )
}
