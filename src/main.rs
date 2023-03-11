use sync_wrapper::SyncWrapper;
extern crate core;

use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Error, format};
use std::ops::Deref;
use std::slice::Chunks;
use std::time::Duration;

use async_recursion::async_recursion;
use axum::{Json, Router, routing::post};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use reqwest::{Client, get};
use rust_tokenizers::tokenizer::{BertTokenizer, Gpt2Tokenizer, Tokenizer};
use rust_tokenizers::vocab::{BpePairVocab, Gpt2Vocab};

use models::{Categories, Category, CategoryWithItems, Item, SortRequestPayload};
use openai::prompt_open_ai;
use crate::models::ErrorResponse;

mod openai;
mod models;

const PROMPT_TEXT_START: &str = "You will receive list of items with titles and id's in form of [title,id].
Based on titles and urls, classify them into categories, by using existing categories or making new ones.";
const PROMPT_TEXT_MIDDLE: &str = "\nValid JSON format to return is:
{ \"categories\": [ { \"category_id\":\"id here\", \"category_name\": \"name here\", \"items\":[tab_id here] } ]}.
Existing categories are:";
const PROMPT_TEXT_ENDING: &str = "A new more detailed list of categories (existing and new) with tabs, in valid JSON format is:";

#[shuttle_service::main]
async fn axum() -> shuttle_service::ShuttleAxum {

    let router = Router::new().route("/sort", post(sort_items));
    let sync_wrapper = SyncWrapper::new(router);

    Ok(sync_wrapper)
}

async fn sort_items(Json(payload): Json<SortRequestPayload>) -> impl IntoResponse {
    let items = payload.items;
    let categories = payload.categories.iter().map(|it| {
        CategoryWithItems {
            category_id: it.id,
            category_name: it.title.to_owned(),
            items: Vec::new(),
        }
    }).collect();

    //make a table of id-to index so we save prompt space
    let item_index: HashMap<_, _> = items.deref().into_iter()
        .enumerate().map(|(i, it)| {
        (i, &it.id)
    }).collect();

    //remap items into values with indexes
    let items_with_indexes = items.iter().map(|it| {
        Item {
            id: *(item_index.iter().find(|(&_, &value)| {
                if value == &it.id {
                    true
                } else { false }
            }).unwrap().0),
            title: it.title.to_owned(),
        }
    }).collect();

    //slice prompt so it fits under 4k tokens max
    let prompt_slices = create_chunks_for_prompting(items_with_indexes);
    let client = Client::builder()
        .http2_keep_alive_timeout(Duration::from_secs(120))
        .timeout(Duration::from_secs(120))//openai api can sometimes freeze
        .build().unwrap();
    sort_recursively(categories, prompt_slices, client).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { message: e })).into_response())
        .map(|wrapper| {
            let new_categories = wrapper.categories.iter().map(|item| {
                CategoryWithItems {
                    category_id: item.category_id.to_owned(),
                    category_name: item.category_name.to_owned(),
                    items: item.items.to_owned(),
                }
            }).collect::<Vec<CategoryWithItems>>();
            (StatusCode::OK, Json(Categories {
                categories: new_categories
            })).into_response()
        })
}

fn find_key_for_value<'a>(map: &'a HashMap<usize, &'a usize>, value: &usize) -> Option<&'a usize> {
    map.iter()
        .find_map(|(key, &val)| if val == value { Some(key) } else { None })
}


#[async_recursion]
async fn sort_recursively(sorted_categories: Vec<CategoryWithItems>, remaining: Vec<Vec<Item>>, client: Client) -> Result<Categories, String> {
    let mut next_categories = Vec::from(sorted_categories.deref());
    let prompt = build_prompt(remaining.first().unwrap().to_vec(),
                              sorted_categories);
    let ai_response_result = prompt_open_ai(prompt, &client).await;
    let res = ai_response_result
        .map_err(|e| format!("Error communicating with OpenAI - {:?}", e))
        .and_then(|ai_response|
            serde_json::from_str::<Categories>(ai_response.as_str())
                .map_err(|_| "Parsing response error".to_string()));
    match res {
        Ok(wrapper) => {
            let mut new_categories = wrapper.categories.to_owned();
            //remove the processed chunk
            let mut next_slice = remaining.to_owned();
            next_slice.remove(0);
            //join the categories
            next_categories.append(&mut new_categories);
            //if we're not done yet recurse
            if next_slice.len() != 0 {
                sort_recursively(next_categories, next_slice, client).await
                    .map_err(|e| format!("Sorting failed, reason: {}", e))
            } else {
                Ok(Categories { categories: next_categories })
            }
        }
        Err(msg) => Err(msg)
    }
}


fn create_chunks_for_prompting(items: Vec<Item>) -> Vec<Vec<Item>> {
    let json_size = serde_json::to_string(&items).unwrap()
        .split_whitespace().collect::<Vec<&str>>().len();
    let hardcoded_prompt = format!("{a}{b}{c}", a = String::from(PROMPT_TEXT_START),
                                   b = String::from(PROMPT_TEXT_MIDDLE),
                                   c = String::from(PROMPT_TEXT_ENDING));
    let hardcoded_prompt_size = hardcoded_prompt.split_whitespace().collect::<Vec<&str>>().len();

    let chunks_to_make = json_size / (2048 - hardcoded_prompt_size);
    let chunk_size: Chunks<Item> = items.chunks(items.len() /
        (if chunks_to_make > 0 { chunks_to_make } else { 1 }));
    return chunk_size.map(|s| s.into()).collect();
}

fn build_prompt(items: Vec<Item>, categories: Vec<CategoryWithItems>) -> String {
    let items_joined = items.iter().map(|item|
        format!("[{title},{id}]",
                title = item.title,
                id = item.id)).collect::<Vec<String>>()
        .join(",");
    let categories_json = serde_json::to_string(&categories).unwrap();
    format!("{prompt}\n{tabs}{middle}{categories}\n{ending}",
            prompt = String::from(PROMPT_TEXT_START),
            tabs = items_joined,
            middle = String::from(PROMPT_TEXT_MIDDLE),
            categories = categories_json,
            ending = String::from(PROMPT_TEXT_ENDING))
}

