use serde::Deserialize;

use serde::Serialize;
use teloxide::{dispatching::dialogue::InMemStorage, dispatching::HandlerExt, prelude::*};

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Deserialize, Serialize)]
struct ApiGeoRequest {
    q: String,
    key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Point {
    lat: f64,
    lng: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeoLocation {
    point: Point,
    country: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeoLocationResponse {
    hits: Vec<GeoLocation>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ApiWeatherRequest {
    lat: f64,
    lon: f64,
    appid: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Weather {
    main: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WeatherMain {
    temp: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct WeatherResponse {
    weather: Option<Vec<Weather>>,
    main: WeatherMain,
}

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveLocationName,
    GeoApiLocationChoice {
        locations: GeoLocationResponse,
    },
    ApiInfoSearch {
        location: GeoLocation,
    },
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");
    let bot = Bot::from_env();
    let handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(dptree::case![State::Start].endpoint(start))
        .branch(dptree::case![State::ReceiveLocationName].endpoint(receive_location_name))
        .branch(
            dptree::case![State::GeoApiLocationChoice { locations }]
                .endpoint(geo_api_location_choice),
        )
        .branch(dptree::case![State::ApiInfoSearch { location }].endpoint(api_info_search));
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Eneter the name of the location")
        .await?;
    dialogue.update(State::ReceiveLocationName).await?;
    Ok(())
}

async fn receive_location_name(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let location_query = match msg.text() {
        Some(text) => text,
        None => {
            bot.send_message(msg.chat.id, "send me plain text.").await?;
            return Ok(());
        }
    };
    let geo_req = ApiGeoRequest {
        q: location_query.to_owned(),
        key: "4aaa52a3-fdb5-467c-b02e-c8c7898fd021".to_string(),
    };
    let client = reqwest::Client::new();
    let url: GeoLocationResponse = client
        .get("https://graphhopper.com/api/1/geocode")
        .query(&geo_req)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mut counter = 1;
    for iter in &url.hits {
        bot.send_message(
            msg.chat.id,
            format!("{counter}. {}", iter.name.clone().unwrap()),
        )
        .await?;
        counter += 1;
    }
    bot.send_message(msg.chat.id, "Select location by intering its number")
        .await?;
    let state = State::GeoApiLocationChoice { locations: url };
    dialogue.update(state).await?;
    Ok(())
}

async fn geo_api_location_choice(
    bot: Bot,
    dialogue: MyDialogue,
    locations: GeoLocationResponse,
    msg: Message,
) -> HandlerResult {
    // pizdec, ToDo: rewrite
    match msg.text().map(|text| text.parse::<usize>()) {
        Some(Ok(number)) => {
            bot.send_message(msg.chat.id, "enter any message to continue")
                .await?;
            dialogue
                .update(State::ApiInfoSearch {
                    location: match locations.hits.get::<usize>(number - 1) {
                        Some(locantion) => locantion.to_owned(),
                        None => {
                            bot.send_message(
                                msg.chat.id,
                                "number out of range, enter valid number.",
                            )
                            .await?;
                            return Ok(());
                        }
                    },
                })
                .await?;
        }
        _ => {
            bot.send_message(
                msg.chat.id,
                "number is not valid enter, enter valid number.",
            )
            .await?;
            return Ok(());
        }
    }
    Ok(())
}

async fn api_info_search(
    bot: Bot,
    dialogue: MyDialogue,
    location: GeoLocation,
    msg: Message,
) -> HandlerResult {
    let weather_req = ApiWeatherRequest {
        lat: location.point.lat,
        lon: location.point.lng,
        appid: "06a6d9beada7f26950314ac2d217c4e0".to_string(),
    };
    let client = reqwest::Client::new();
    let weather: WeatherResponse = client
        .get("https://api.openweathermap.org/data/2.5/weather")
        .query(&weather_req)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let weather_description = match weather.weather {
        Some(desc) => desc.get(0).unwrap().main.clone(),
        None => "".to_string(),
    };
    bot.send_message(
        msg.chat.id,
        format!(
            "the weather is {}\nand the temperature is {}",
            weather_description,
            weather.main.temp - 273.15
        ),
    )
    .await?;
    dialogue.update(State::Start).await?;
    Ok(())
}
