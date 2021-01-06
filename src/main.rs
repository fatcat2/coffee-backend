use serde::{Deserialize, Serialize};
use postgres::{Client, NoTls};
use warp::Filter;
use std::env;
use http::StatusCode;
use dirs::home_dir;

#[derive(Deserialize, Serialize)]
struct Coffee {
    token: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct CoffeeDay {
    day: String,
    cups: i32
}

#[derive(Deserialize, Serialize)]
struct Bad {
    message: String,
}



fn authenticate(token: &str) -> Result<bool, postgres::Error>{
    let postgres_username: &str = &env::var("POSTGRES_USERNAME").expect("MISSING POSTGRES USERNAME ENV VAR");
    let postgres_password: &str = &env::var("POSTGRES_PASSWORD").expect("MISSING POSTGRES PASSWORD ENV VAR");
    let postgres_database: &str = &env::var("POSTGRES_DATABASE").expect("MISSING POSTGRES DATABASE ENV VAR");
    let postgres_host: &str = &env::var("POSTGRES_HOST").expect("MISSING POSTGRES HOST ENV VAR");
    

    let mut client = Client::configure()
        .user(&postgres_username)
        .password(&postgres_password)
        .dbname(&postgres_database)
        .host(&postgres_host)
        .connect(NoTls)?;

    let postgres_row = client
        .query_one("select token from users where token=$1::TEXT", &[&token])?;

    let postgres_token: &str = postgres_row.get(0);
    
    if postgres_token == token {
        return Ok(true)
    }else {
        return Ok(false)
    }   
}

fn increment_coffee() -> Result<bool, postgres::Error> {
    let postgres_username: &str = &env::var("POSTGRES_USERNAME").expect("MISSING POSTGRES USERNAME ENV VAR");
    let postgres_password: &str = &env::var("POSTGRES_PASSWORD").expect("MISSING POSTGRES PASSWORD ENV VAR");
    let postgres_database: &str = &env::var("POSTGRES_DATABASE").expect("MISSING POSTGRES DATABASE ENV VAR");
    let postgres_host: &str = &env::var("POSTGRES_HOST").expect("MISSING POSTGRES HOST ENV VAR");
    

    let mut client = Client::configure()
        .user(&postgres_username)
        .password(&postgres_password)
        .dbname(&postgres_database)
        .host(&postgres_host)
        .connect(NoTls)?;

    let postgres_statement = client.prepare("insert into coffee_data (day, cups) values (current_date, 1) on conflict (day) do update set cups = coffee_data.cups + 1 where coffee_data.day = current_date;")?;
    client.execute(&postgres_statement, &[])?;

    return Ok(true);
}

fn data_dump() -> Result<Vec<CoffeeDay>, postgres::Error> {
    let postgres_username: &str = &env::var("POSTGRES_USERNAME").expect("MISSING POSTGRES USERNAME ENV VAR");
    let postgres_password: &str = &env::var("POSTGRES_PASSWORD").expect("MISSING POSTGRES PASSWORD ENV VAR");
    let postgres_database: &str = &env::var("POSTGRES_DATABASE").expect("MISSING POSTGRES DATABASE ENV VAR");
    let postgres_host: &str = &env::var("POSTGRES_HOST").expect("MISSING POSTGRES HOST ENV VAR");
    

    let mut client = Client::configure()
        .user(postgres_username)
        .password(postgres_password)
        .dbname(postgres_database)
        .host(postgres_host)
        .connect(NoTls)?;

    let postgres_statement = client.prepare("select to_char(day, 'Mon dd, YYYY'), cups from coffee_data")?;
    let rows = client.query(&postgres_statement, &[])?;

    println!("{:?}", rows);

    let coffee_days: Vec<CoffeeDay> = rows.iter().map(|row| {
        CoffeeDay{
            day: row.get(0),
            cups: row.get(1)
        }
    }).collect();

    println!("{:?}", coffee_days);

    return Ok(coffee_days);

}

#[tokio::main]
async fn main() {
    println!("good morning");

    let mut index_path = match home_dir() {
        Some(value) => value,
        _ => panic!("SHIT GONE WRONG NO COffEE FOLDER")
    };
    index_path.push(".coffee-server");
    index_path.push("index");
    index_path.set_extension("html");


    let cors = warp::cors().allow_any_origin();    

    let hello_world = warp::get().and(warp::path::end()).and(warp::fs::file("/home/ryan/.coffee-server/index.html"));
    
    let drink = warp::post()
        .and(warp::path("drink"))
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .map(|coffee: Coffee| {
            // Authenticate
            match authenticate(coffee.token.as_str()){
                Ok(true) => match increment_coffee() {
                    Ok(true) => warp::reply::with_status("ok", StatusCode::OK),
                    Err(e) => {
                        println!("{:?}", e);
                        warp::reply::with_status("server error", StatusCode::INTERNAL_SERVER_ERROR)
                    },
                    _ => warp::reply::with_status("server error", StatusCode::INTERNAL_SERVER_ERROR)
                },
                Ok(false) => warp::reply::with_status("UNAUTHORIZED", StatusCode::UNAUTHORIZED),
                _ => warp::reply::with_status("server error", StatusCode::INTERNAL_SERVER_ERROR)
            }
        });
    
    let data_route = warp::get()
        .and(warp::path("data"))
        .map(|| {
            println!("enter");
            match data_dump(){
                Ok(coffee_days) => warp::reply::json(&coffee_days),
                Err(e) => {
                    println!("{:?}", e);
                    warp::reply::json(&Bad{message:String::from("bad")})
                },
            }
            // warp::reply()
        }).with(cors);

    let routes = drink.or(data_route).or(hello_world);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}