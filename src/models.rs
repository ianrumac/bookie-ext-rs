use serde::{Deserialize, Serialize};


#[derive(Deserialize)]
pub(crate) struct SortRequestPayload {
    pub(crate) categories: Vec<Category>,
    pub(crate) items: Vec<Bookmark>,
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
pub(crate) struct Bookmark {
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
    pub id: usize,
    pub title: String,
    pub items: Vec<usize>
}