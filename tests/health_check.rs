use sqlx::{PgConnection, Connection};
use std::net::TcpListener;
use reqwest;
use zero2prod::configuration::get_configuration;


fn spawn_app() -> String{
    let host = "127.0.0.1";
    let listener = TcpListener::bind(format!("{}:{}", host, "0")).expect("Failed to bind random porto");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::startup::run(listener).expect("Failed to bind address");
    
    let _ = tokio::spawn(server);

    format!("http://{}:{}", host, port)
}

#[tokio::test]
async fn health_check_works() {
    let url = spawn_app();

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", url))
        .send()
        .await
        .expect("Failted to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_return_a_200_for_valid_form_data() {
    let app_address = spawn_app();
    let configuration = get_configuration().expect("Failed to read configuration,");
    let connection_string = configuration.database.connection_string();
    
    let mut connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connecto to Postgres.");

    let client = reqwest::Client::new();

    let body = "name=test%20test&email=test%domain.com";
    let response = client
        .post(format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions", )
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved subscription.");
    
    assert_eq!(saved.email, "test@domain.com");
    assert_eq!(saved.name, "test test");

}

#[tokio::test]
async fn subscribe_return_a_400_when_data_is_missing() {
    let app_address = spawn_app();
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=test%20test", "missing the mail"),
        ("email=test%40domain.com", "missing the name"),
        ("", "missing both name and email")
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", &app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");
        
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}