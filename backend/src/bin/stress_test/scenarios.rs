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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_order_write() {
        let s = get_scenario("order_write").unwrap();
        assert_eq!(s.name, "order_write");
        assert_eq!(s.scenario_type, "http");
        assert_eq!(s.method, "POST");
        assert_eq!(s.endpoint, "/api/v1/orders");
        assert!(s.body.is_some());
    }

    #[test]
    fn test_scenario_order_read() {
        let s = get_scenario("order_read").unwrap();
        assert_eq!(s.name, "order_read");
        assert_eq!(s.scenario_type, "http");
        assert_eq!(s.method, "GET");
        assert!(s.body.is_none());
    }

    #[test]
    fn test_scenario_portfolio() {
        let s = get_scenario("portfolio").unwrap();
        assert_eq!(s.name, "portfolio");
        assert_eq!(s.scenario_type, "http");
        assert_eq!(s.method, "GET");
        assert_eq!(s.endpoint, "/api/v1/portfolio/positions");
    }

    #[test]
    fn test_scenario_health() {
        let s = get_scenario("health").unwrap();
        assert_eq!(s.name, "health");
        assert_eq!(s.scenario_type, "http");
        assert_eq!(s.method, "GET");
        assert_eq!(s.endpoint, "/health");
        assert!(s.body.is_none());
    }

    #[test]
    fn test_scenario_unknown() {
        assert!(get_scenario("unknown_scenario").is_none());
    }

    #[test]
    fn test_list_scenarios() {
        let scenarios = list_scenarios();
        assert!(scenarios.contains(&"order_write"));
        assert!(scenarios.contains(&"order_read"));
        assert!(scenarios.contains(&"portfolio"));
        assert!(scenarios.contains(&"health"));
    }
}
