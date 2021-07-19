use database::establish_connection;

fn main() {
    println!("Attempting to connect to database");
    match establish_connection() {
        Ok(_) => println!("Connected to database"),
        Err(error) => panic!("Failed to connect to database: {}", error),
    }
}
