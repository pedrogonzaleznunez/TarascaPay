use serde::{Deserialize, Serialize};

// --------------------- User DTOs ---------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserRequest {
    pub username: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserResponse {
    pub id: u64,
    pub username: String,
    pub status: String,
}

