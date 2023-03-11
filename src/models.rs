use serde::{Deserialize, Serialize};


#[derive(Deserialize)]
pub(crate) struct SortRequestPayload {
    pub(crate) categories: Vec<Category>,
    pub(crate) items: Vec<Item>,
}

#[derive(Deserialize)]
#[derive(Serialize)]
#[derive(Clone)]
pub(crate) struct Category {
    pub(crate) id: usize,
    pub(crate) title: String,
}

#[derive(Deserialize)]
#[derive(Serialize)]
#[derive(Clone)]
pub(crate) struct Item {
    pub(crate) id: usize,
    pub(crate) title: String,
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub(crate) struct Categories {
    pub categories: Vec<CategoryWithItems>
}

#[derive(Deserialize)]
#[derive(Serialize)]
#[derive(Clone)]
pub(crate) struct CategoryWithItems {
    pub category_id: usize,
    pub category_name: String,
    pub items: Vec<usize>
}

#[derive(Serialize)]
pub(crate) struct ErrorResponse {
    pub message: String,
}