use crate::helper::TestAppBuilder;

#[actix_web::test]
async fn health_check() {
    let mut app = TestAppBuilder::new().build();
    app.spawn_app().await;
    let path = app.path("health_check");
    let response = reqwest::Client::new()
        .get(&path)
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(response.status(), 200);
}
