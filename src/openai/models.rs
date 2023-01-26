use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[derive(Serialize)]
pub(crate) struct AskGPT {
    pub prompt: String,
    pub model: String,
    pub max_tokens: usize,
    pub stream: bool,
    pub temperature: usize,
    pub top_p: usize,
    pub n: usize,
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub(crate) struct Completion {
    pub model: String,
    pub choices: Vec<Choices>,
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub(crate) struct Choices {
    pub text: String,
    pub index: usize,
}

