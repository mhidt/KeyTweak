use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: &str = "1.0";
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;
pub const MAX_TEXT_LENGTH: usize = 10000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Init {
        protocol_version: String,
    },
    Translate {
        id: u64,
        q: String,
        source: String,
        target: String,
    },
    Status {
        id: u64,
    },
    Exit {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub response_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub recoverable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl ErrorInfo {
    pub fn model_not_found(source: &str, target: &str) -> Self {
        Self {
            code: "MODEL_NOT_FOUND".to_string(),
            message: format!("Translation model {source}\u{2192}{target} is not installed"),
            recoverable: false,
            hint: Some("Reinstall KeyTweak with translator component".to_string()),
        }
    }

    pub fn text_too_long(len: usize) -> Self {
        Self {
            code: "TEXT_TOO_LONG".to_string(),
            message: format!("Text length ({len}) exceeds maximum ({MAX_TEXT_LENGTH})"),
            recoverable: true,
            hint: None,
        }
    }

    pub fn invalid_request(msg: &str) -> Self {
        Self {
            code: "INVALID_REQUEST".to_string(),
            message: msg.to_string(),
            recoverable: true,
            hint: None,
        }
    }

    pub fn translation_error(msg: &str) -> Self {
        Self {
            code: "TRANSLATION_ERROR".to_string(),
            message: msg.to_string(),
            recoverable: true,
            hint: None,
        }
    }
}

impl Response {
    pub fn init(ready: bool, languages: Vec<String>) -> Self {
        Self {
            response_type: Some("init".to_string()),
            protocol_version: Some(PROTOCOL_VERSION.to_string()),
            ready: Some(ready),
            capabilities: Some(vec!["translate".to_string(), "status".to_string()]),
            languages: if ready { Some(languages) } else { None },
            ..Default::default()
        }
    }

    pub fn init_error(code: &str, message: &str) -> Self {
        Self {
            response_type: Some("init".to_string()),
            protocol_version: Some(PROTOCOL_VERSION.to_string()),
            ready: Some(false),
            error: Some(ErrorInfo {
                code: code.to_string(),
                message: message.to_string(),
                recoverable: false,
                hint: None,
            }),
            ..Default::default()
        }
    }

    pub fn translate_result(id: u64, translated: String) -> Self {
        Self {
            id: Some(id),
            translated: Some(translated),
            ..Default::default()
        }
    }

    pub fn translate_error(id: u64, error: ErrorInfo) -> Self {
        Self {
            id: Some(id),
            error: Some(error),
            ..Default::default()
        }
    }

    pub fn status(id: u64, ready: bool, languages: Vec<String>) -> Self {
        Self {
            id: Some(id),
            ready: Some(ready),
            languages: Some(languages),
            ..Default::default()
        }
    }

    pub fn heartbeat(timestamp: f64) -> Self {
        Self {
            response_type: Some("heartbeat".to_string()),
            timestamp: Some(timestamp),
            ..Default::default()
        }
    }

    pub fn shutdown(reason: &str) -> Self {
        Self {
            response_type: Some("shutdown".to_string()),
            reason: Some(reason.to_string()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_translate_request() {
        let req = Request::Translate {
            id: 42,
            q: "Hello".to_string(),
            source: "en".to_string(),
            target: "ru".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"cmd\":\"translate\""));
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"q\":\"Hello\""));
    }

    #[test]
    fn deserialize_translate_response() {
        let json = r#"{"id":42,"translated":"Привет"}"#;
        let resp: Response = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(42));
        assert_eq!(resp.translated, Some("Привет".to_string()));
    }

    #[test]
    fn deserialize_init_response() {
        let json = r#"{"type":"init","protocol_version":"1.0","ready":true,"capabilities":["translate","status"]}"#;
        let resp: Response = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response_type, Some("init".to_string()));
        assert_eq!(resp.protocol_version, Some("1.0".to_string()));
        assert_eq!(resp.ready, Some(true));
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{"id":1,"error":{"code":"MODEL_NOT_FOUND","message":"Not found","recoverable":false}}"#;
        let resp: Response = serde_json::from_str(json).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, "MODEL_NOT_FOUND");
        assert!(!err.recoverable);
    }

    #[test]
    fn deserialize_heartbeat() {
        let json = r#"{"type":"heartbeat","timestamp":1234567890.0}"#;
        let resp: Response = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response_type, Some("heartbeat".to_string()));
        assert!(resp.id.is_none());
    }
}
