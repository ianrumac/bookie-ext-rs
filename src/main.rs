extern crate core;

use std::borrow::{Borrow};
use std::collections::HashMap;
use std::fmt::Error;
use std::ops::Deref;
use std::time::Duration;

use async_recursion::async_recursion;
use axum::{Json, Router, routing::post};
use reqwest::{Client};

use models::{Bookmark, Categories, Category, CategoryWithItems, SortRequestPayload};
use openai::prompt_open_ai;

mod openai;
mod models;

const PROMPT_TEXT: &str = "You will receive list of bookmarks with titles.Based on titles and urls,
 classify them into categories and reuse existing ones. Tabs are:";
const PROMPT_TEXT_APPEND: &str = "\nJSON format to return is:
{ categories: [ { \"category_id\":\"idhere\", \"category_name\": \"name here\", \"items\":[tab_id here] } ]}.
Existing categories are:";
const PROMPT_TEXT_ENDING: &str = "A new more detailed list of categories (existing and new) with tabs:";

#[tokio::main]
async fn main() {
    //env_logger::init();
    let server = Router::new().route("/sort", post(sort_bookmarks));

    axum::Server::bind(&"0.0.0.0:8001".parse().unwrap())
        .serve(server.into_make_service()).await.unwrap()
}

async fn sort_bookmarks(Json(payload): Json<SortRequestPayload>) {
    let items = payload.items;
    let categories = payload.categories.iter().map(|it| {
        let old = it.to_owned();
        CategoryWithItems {
            id: old.id,
            title: old.title,
            items: Vec::new(),
        }
    }).collect();


    //make a table of id-to index so we save prompt space
    let item_index: HashMap<_, _> = items.deref().into_iter()
        .enumerate().map(|(i, it)| {
        (i, it.id)
    }).collect();

    //remap items into values with indexes
    let items_with_indexes = items.iter().map(|it| {
        Bookmark {
            id: *(item_index.iter().find(|(&_, &value)| {
                if value == it.id {
                    true
                } else { false }
            }).unwrap().0),
            title: it.title.to_owned(),
        }
    }).collect();

    //slice prompt so it fits under 4k tokens max
    let prompt_slices = create_chunks_for_prompting(items_with_indexes);
    let client = Client::builder().http2_keep_alive_timeout(Duration::from_secs(120))
        .timeout(Duration::from_secs(120))//openai api can sometimes freeze
        .pool_max_idle_per_host(0).build().unwrap();
    let res = sort_recursively(categories, prompt_slices, client).await.unwrap();

    //todo generate response
}

#[async_recursion]
async fn sort_recursively(categories: Vec<CategoryWithItems>, remaining: Vec<Vec<Bookmark>>, client: Client) -> Result<Categories, Error> {
    println!("Entering recursion");
    let current_categories = categories;
    let mut next_categories = Vec::from(current_categories.deref());
    let prompt = build_prompt(remaining.first().unwrap().to_vec(), current_categories);
    let ai_response = prompt_open_ai(prompt, client.borrow()).await.unwrap();
    println!("Entering recursion - passed prompt");
    let json = snailquote::unescape(ai_response.as_str()).unwrap();
    let wrapper = serde_json::from_str::<Categories>(json.as_str()).unwrap();
    let mut new_categories = wrapper.categories;
    let mut next_slice = remaining.to_owned();
    next_slice.remove(0);
    next_categories.append(&mut new_categories);
    println!("RES:{}", json);
    if next_slice.len() != 0 {
        let next = sort_recursively(next_categories, next_slice, client.to_owned()).await;
        Ok(next.unwrap())
    } else {
        Ok(Categories { categories: next_categories })
    }
}


fn create_chunks_for_prompting(items: Vec<Bookmark>) -> Vec<Vec<Bookmark>> {
    let json_size = serde_json::to_string(&items).unwrap()
        .replace("http://", "").replace("https://", "").len();
    let mut chunks_to_make = (json_size / 3) / 2000;
    if chunks_to_make == 0 {
        chunks_to_make = 1;
    }
    let chunk_size = items.chunks(items.len() / chunks_to_make);
    println!("Json size would be {}, splitting by factor of {} we get chunks of {}", json_size, chunks_to_make, items.len() / chunks_to_make);
    return chunk_size.map(|s| s.into()).collect();
}

fn build_prompt(items: Vec<Bookmark>, categories: Vec<CategoryWithItems>) -> String {
    let items_json = serde_json::to_string(&items).unwrap();
    println!("Items = {}", items_json);
    let groups_json = serde_json::to_string(&categories).unwrap();
    let txt = format!("{prompt}\n{tabs}{middle}{categories}\n{ending}",
                      prompt = PROMPT_TEXT,
                      tabs = items_json, middle = PROMPT_TEXT_APPEND, categories = groups_json, ending = PROMPT_TEXT_ENDING);
    println!("Prompt size after = {}", txt.replace("http://", "").replace("https://", "").len() / 3);
    txt
}

