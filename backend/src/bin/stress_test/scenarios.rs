pub struct Scenario {
    pub name: String,
    pub scenario_type: String,
    pub method: String,
    pub endpoint: String,
    pub body: Option<String>,
}

pub fn get_scenario(name: &str) -> Option<Box<Scenario>> {
    match name {
        "order_write" => Some(Box::new(Scenario {
            name: "order_write".to_string(),
            scenario_type: "http".to_string(),
            method: "POST".to_string(),
            endpoint: "/api/v1/orders".to_string(),
            body: Some(
                r#"{"symbol":"BTCUSDT","side":"buy","order_type":"market","quantity":"0.001"}"#
                    .to_string(),
            ),
        })),
        "order_read" => Some(Box::new(Scenario {
            name: "order_read".to_string(),
            scenario_type: "http".to_string(),
            method: "GET".to_string(),
            endpoint: "/api/v1/orders?page=1&size=20".to_string(),
            body: None,
        })),
        "portfolio" => Some(Box::new(Scenario {
            name: "portfolio".to_string(),
            scenario_type: "http".to_string(),
            method: "GET".to_string(),
            endpoint: "/api/v1/portfolio/positions".to_string(),
            body: None,
        })),
        "health" => Some(Box::new(Scenario {
            name: "health".to_string(),
            scenario_type: "http".to_string(),
            method: "GET".to_string(),
            endpoint: "/health".to_string(),
            body: None,
        })),
        _ => None,
    }
}

pub fn list_scenarios() -> Vec<&'static str> {
    vec!["order_write", "order_read", "portfolio", "health"]
}
