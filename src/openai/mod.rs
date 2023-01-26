use std::borrow::Borrow;
use std::env;
use std::error::Error;

use reqwest::{Client, StatusCode};

use models::{AskGPT, Choices, Completion};

mod models;

pub(crate) async fn prompt_open_ai(txt: String, client: &Client) -> Result<String, String> {
    let len = txt.len();
    let token = env::var("AI_TOKEN").expect("Need client token to run");
    let auth_header = format!("Bearer {}",token);
    println!("Asking openAI about text with length {} ({})", len, len / 3);
    let req = client.post("https://api.openai.com/v1/completions")
        .header("Authorization", auth_header)
        .json(&AskGPT {
            prompt: txt,
            model: String::from("text-davinci-003"),
            max_tokens: 4096 - len / 3,
            top_p: 1,
            n: 1,
            stream: false,
            temperature: 0,
        }).send().await;
    return match req {
        Ok(it) => {
            return match it.status() {
                StatusCode::OK => {
                    return match it.json::<Completion>().await {
                        Ok(parsed) => {
                            //there is always at least 1 due to our request
                            let choices = parsed.choices.first().unwrap();
                            let json: &str = choices.text.borrow();
                            Ok(String::from(json))
                        }
                        Err(err) =>
                            {
                                return Err(String::from("Error parsing"));
                            }
                    };
                }
                other => {
                    return Err(String::from(format!("Error in HTTP - code {}", other.as_str())));
                }
            };
        }
        Err(err) => {
            return Err(String::from(format!("Beep boop generic error - {}", err.status().unwrap().as_str())));
        }
    };
}
