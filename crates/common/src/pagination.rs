use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PageRequest {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    0
}

fn default_per_page() -> i64 {
    20
}

impl PageRequest {
    pub fn offset(&self) -> i64 {
        self.page * self.per_page
    }

    pub fn limit(&self) -> i64 {
        self.per_page
    }
}

#[derive(Debug, Serialize)]
pub struct PageResult<T: Serialize> {
    pub data: Vec<T>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}

impl<T: Serialize> PageResult<T> {
    pub fn new(data: Vec<T>, page: i64, per_page: i64, total: i64) -> Self {
        Self {
            data,
            page,
            per_page,
            total,
        }
    }
}
