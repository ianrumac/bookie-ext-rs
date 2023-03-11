use std::borrow::Borrow;
use std::env;
use std::error::Error;
use std::fmt::Display;

use reqwest::{Client, StatusCode};

use models::{AskGPT, Choices, Completion};

mod models;

pub(crate) async fn prompt_open_ai(txt: String, client: &Client) -> Result<String, String> {
    let token = String::from("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"); //env::var("AI_TOKEN").expect("Need client token to run");
    let auth_header = format!("Bearer {}", token);
    let req = client.post("https://api.openai.com/v1/completions")
        .header("Authorization", auth_header)
        .json(&AskGPT {
            prompt: txt,
            model: String::from("text-davinci-003"),
            max_tokens: 4096,
            top_p: 1,
            n: 1,
            stream: false,
            temperature: 0,
        }).send().await;
    match req {
        Ok(response) => {
            match response.status() {
                StatusCode::OK => {
                    match response.json::<Completion>().await {
                        Ok(parsed) => {
                            //there is always at least 1 due to our request
                            let choices = parsed.choices.first().unwrap();
                            let json: &str = choices.text.borrow();
                            Ok(String::from(json))
                        }
                        _ => Err(String::from("Error parsing"))
                    }
                }
                _ => Err(String::from("Error connecting"))
            }
        }
        _ => Err(String::from("Error parsing"))
    }
}
