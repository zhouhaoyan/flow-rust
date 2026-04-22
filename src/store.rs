use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use crate::models::ParsedPlantData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantRecord {
    pub id: i64,
    pub original_text: String,
    // Parsed fields
    pub event_type: Option<String>,
    pub plant_short_name: Option<String>,
    pub event_date: Option<String>,
    pub quantity_location: Option<String>,
    pub batch: Option<String>,
    pub details: Option<String>,
    pub record_type: Option<String>,
    pub plant_name: Option<String>,
    pub is_germination_report: bool,
    pub is_death_report: bool,
    pub is_cumulative_quantity: bool,
    pub confidence: f32,
    pub parsing_errors: Vec<String>,
    // Status
    pub confirmed: bool,
    pub created_at: DateTime<Utc>,
}

// Keep PlantData for backward compatibility (used by old DeepSeek parsing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantData {
    pub plant_type: Option<String>,
    pub action: Option<String>,
    pub quantity: Option<i32>,
    pub notes: Option<String>,
}

pub struct Store {
    records: Mutex<Vec<PlantRecord>>,
    next_id: Mutex<i64>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            records: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }

    pub async fn add_record(&self, original_text: String, parsed_data: Option<ParsedPlantData>) -> Result<i64, String> {
        let mut records = self.records.lock().await;
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;

        let record = if let Some(data) = parsed_data {
            PlantRecord {
                id,
                original_text,
                event_type: data.event_type,
                plant_short_name: data.plant_short_name,
                event_date: data.event_date,
                quantity_location: data.quantity_location,
                batch: data.batch,
                details: data.details,
                record_type: data.record_type,
                plant_name: data.plant_name,
                is_germination_report: data.is_germination_report,
                is_death_report: data.is_death_report,
                is_cumulative_quantity: data.is_cumulative_quantity,
                confidence: data.confidence,
                parsing_errors: data.parsing_errors,
                confirmed: false,
                created_at: Utc::now(),
            }
        } else {
            PlantRecord {
                id,
                original_text,
                event_type: None,
                plant_short_name: None,
                event_date: None,
                quantity_location: None,
                batch: None,
                details: None,
                record_type: None,
                plant_name: None,
                is_germination_report: false,
                is_death_report: false,
                is_cumulative_quantity: false,
                confidence: 0.0,
                parsing_errors: Vec::new(),
                confirmed: false,
                created_at: Utc::now(),
            }
        };

        records.push(record);
        Ok(id)
    }

    pub async fn confirm_record(&self, id: i64) -> Result<bool, String> {
        let mut records = self.records.lock().await;
        if let Some(record) = records.iter_mut().find(|r| r.id == id) {
            record.confirmed = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn get_record(&self, id: i64) -> Result<Option<PlantRecord>, String> {
        let records = self.records.lock().await;
        Ok(records.iter().find(|r| r.id == id).cloned())
    }

    pub async fn list_records(&self) -> Result<Vec<PlantRecord>, String> {
        let records = self.records.lock().await;
        Ok(records.clone())
    }
}