use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Task {
    pub id: u32,
    pub description: String,
    pub completed: bool
}