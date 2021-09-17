use std::env;

use dotenv::dotenv;
use rocket_db_pools::Database;
use rocket::{figment::map, fs::FileServer, get, launch, response::Redirect, routes};

#[derive(Debug, Database)]
#[database("battlefox_web")]
pub struct Db(mongodb::Client);

// #[get("/")]
// async fn index() -> Redirect {
//     Redirect::temporary("/react-app")
// }

/// This is `fn main() { ... }` but needlessly spiced up with macros.
#[launch]
fn rocket() -> _ {
    dotenv().ok();

    let db_url = env::var("MONGO_DB").expect("Please specify the `MONGO_DB` env var.");

    let figment = rocket::Config::figment()
        .merge(("address", "0.0.0.0"))
        .merge(("databases.battlefox_web", map!["url" => db_url]));

    rocket::custom(figment)
        .attach(Db::init())
        // .mount("/", routes![index])
        .mount("/", FileServer::from("frontend/build"))
}
