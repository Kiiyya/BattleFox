use rocket_db_pools::Database;
use rocket::launch;

#[derive(Debug, Database)]
#[database("bfox_web")]
pub struct Db(mongodb::Client);

/// This is `fn main() { ... }` but needlessly spiced up with macros.
#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Db::init())
}
