//! Query bodies for the list endpoints (POST variants, so values with special characters are safe).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VariableCriterion {
    pub name: String,
    /// eq | neq | gt | gteq | lt | lteq | like | notLike
    pub operator: String,
    pub value: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sorting {
    pub sort_by: String,
    pub sort_order: String, // asc | desc
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInstanceQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_instance_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_key_like: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_definition_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_definition_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_definition_key_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub super_process_instance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_process_instance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_incident: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incident_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incident_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incident_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incident_message_like: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_id_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_process_instances: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaf_process_instances: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<VariableCriterion>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_names_ignore_case: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_values_ignore_case: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<Vec<Sorting>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoricProcessInstanceQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_instance_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_instance_business_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_instance_business_key_like: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_definition_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_definition_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_definition_key_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_definition_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub super_process_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_process_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_process_instances: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unfinished: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_incidents: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_root_incidents: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incident_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incident_status: Option<String>, // open | resolved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incident_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incident_message_like: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub externally_terminated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internally_terminated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_activity_id_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_activity_id_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_activity_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_activity_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<VariableCriterion>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_names_ignore_case: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_values_ignore_case: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<Vec<Sorting>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VariableInstanceQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_name_like: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_instance_id_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_instance_id_in: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_values: Option<Vec<VariableCriterion>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<Vec<Sorting>>,
}

/// Body for `/process-instance/{id}/modification`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModificationRequest {
    pub instructions: Vec<crate::types::ModificationInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_custom_listeners: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_io_mappings: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
}

/// Body for `/process-definition/{id}/restart` (restart finished instances).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RestartRequest {
    pub process_instance_ids: Vec<String>,
    pub instructions: Vec<crate::types::ModificationInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_variables: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_custom_listeners: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_io_mappings: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub without_business_key: Option<bool>,
}
