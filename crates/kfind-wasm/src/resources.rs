use js_sys::{Object, Reflect, Uint8Array};
use kfind::{Engine, ResourceBundle};
use wasm_bindgen::{JsCast, JsError, JsValue};

const RESOURCE_FIELDS: [&str; 3] = ["component", "enrichedPredicates", "fullPos"];

pub fn engine_from_resources(value: JsValue) -> Result<Engine, JsError> {
    let object = value
        .dyn_ref::<Object>()
        .ok_or_else(|| resource_error("expected an object"))?;
    validate_resource_fields(object)?;

    let full_pos = optional_bytes(object, "fullPos")?;
    let enriched_predicates = optional_string(object, "enrichedPredicates")?;
    let component = optional_bytes(object, "component")?;

    Engine::with_resources(ResourceBundle {
        full_pos: full_pos.as_deref(),
        enriched_predicates: enriched_predicates.as_deref(),
        component,
    })
    .map_err(super::initialization_error)
}

fn validate_resource_fields(object: &Object) -> Result<(), JsError> {
    for key in Object::keys(object).iter() {
        let key = key
            .as_string()
            .ok_or_else(|| resource_error("resource keys must be strings"))?;
        if RESOURCE_FIELDS.binary_search(&key.as_str()).is_err() {
            return Err(resource_error(&format!("unknown field `{key}`")));
        }
    }
    Ok(())
}

fn optional_bytes(object: &Object, field: &str) -> Result<Option<Vec<u8>>, JsError> {
    let value = Reflect::get(object, &JsValue::from_str(field))
        .map_err(|_| resource_error(&format!("failed to read `{field}`")))?;
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    value
        .dyn_into::<Uint8Array>()
        .map(|bytes| Some(bytes.to_vec()))
        .map_err(|_| resource_error(&format!("`{field}` must be a Uint8Array")))
}

fn optional_string(object: &Object, field: &str) -> Result<Option<String>, JsError> {
    let value = Reflect::get(object, &JsValue::from_str(field))
        .map_err(|_| resource_error(&format!("failed to read `{field}`")))?;
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    value
        .as_string()
        .map(Some)
        .ok_or_else(|| resource_error(&format!("`{field}` must be a string")))
}

fn resource_error(message: &str) -> JsError {
    JsError::new(&format!("invalid kfind resources: {message}"))
}
