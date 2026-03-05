use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use chrono::Datelike;
use quest_log::{database::Database, handlers, state::AppState};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

fn get_css_content() -> String {
    std::fs::read_to_string("static/style.css").expect("Failed to read CSS file")
}

// ===== PIXEL ART & GLOW CSS INTEGRATION TESTS =====

#[tokio::test]
async fn test_pixel_font_exists() {
    let font_path = std::path::Path::new("static/fonts/PressStart2P.woff2");
    assert!(
        font_path.exists(),
        "Pixel font file should exist at {}",
        font_path.display()
    );

    let font_size = std::fs::metadata(font_path).expect("Failed to get font metadata");
    assert!(
        font_size.len() > 1000,
        "Font file should be larger than 1KB"
    );

    println!("Pixel font exists integration test completed successfully");
}

#[tokio::test]
async fn test_css_glow_properties_present() {
    let css = get_css_content();

    assert!(
        css.contains("--glow-intensity"),
        "CSS should contain glow intensity variable"
    );
    assert!(
        css.contains("--shadow-glow-primary"),
        "CSS should contain primary glow shadow"
    );
    assert!(
        css.contains("--shadow-glow-success"),
        "CSS should contain success glow shadow"
    );
    assert!(
        css.contains("--shadow-glow-warning"),
        "CSS should contain warning glow shadow"
    );

    println!("CSS glow properties integration test completed successfully");
}

#[tokio::test]
async fn test_glow_animations_defined() {
    let css = get_css_content();

    assert!(
        css.contains("@keyframes glow-pulse-success"),
        "CSS should contain glow-pulse-success animation"
    );
    assert!(
        css.contains("@keyframes glow-pulse-warning"),
        "CSS should contain glow-pulse-warning animation"
    );
    assert!(
        css.contains("@keyframes glow-burst"),
        "CSS should contain glow-burst animation"
    );
    assert!(
        css.contains("@keyframes completed-glow"),
        "CSS should contain completed-glow animation"
    );

    println!("Glow animations integration test completed successfully");
}

#[tokio::test]
async fn test_pixel_font_face_defined() {
    let css = get_css_content();

    assert!(
        css.contains("@font-face"),
        "CSS should contain @font-face declaration"
    );
    assert!(
        css.contains("PressStart2P.woff2"),
        "CSS should reference Press Start 2P font"
    );

    println!("Pixel font face integration test completed successfully");
}

#[tokio::test]
async fn test_quest_page_contains_glow_elements() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = chrono::Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = quest_log::models::CreateQuestRequest {
        title: "Glow Test Quest".to_string(),
        description: Some("Testing glow effects".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    db.create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let app = Router::new()
        .route("/", get(handlers::quests))
        .route(
            "/quests/toggle",
            axum::routing::post(handlers::toggle_quest),
        )
        .with_state(AppState {
            db,
            bcast: broadcast::channel::<handlers::ServerMessage>(128).0,
        });

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("stats-panel"),
        "Page should contain stats-panel element"
    );
    assert!(
        body_str.contains("Glow Test Quest"),
        "Page should contain quest title"
    );
    assert!(body_str.contains("25 EXP"), "Page should contain EXP value");
    assert!(
        body_str.contains("Mark Complete"),
        "Page should contain toggle button"
    );
    assert!(
        body_str.contains("/style.css"),
        "Page should reference CSS file"
    );

    println!("Quest page glow elements integration test completed successfully");
}

#[tokio::test]
async fn test_completed_quest_has_glow_class() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = chrono::Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = quest_log::models::CreateQuestRequest {
        title: "Complete Me".to_string(),
        description: None,
        exp_value: Some(15),
        day_of_week,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let app = Router::new()
        .route("/", get(handlers::quests))
        .route(
            "/quests/toggle",
            axum::routing::post(handlers::toggle_quest),
        )
        .with_state(AppState {
            db,
            bcast: broadcast::channel::<handlers::ServerMessage>(128).0,
        });

    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("quest-item completed"),
        "Completed quest should have completed class for glow effects"
    );
    assert!(
        body_str.contains("✅ Quest Complete"),
        "Quest should show completed status"
    );
    assert!(
        body_str.contains("expToday"),
        "Response should patch expToday signal"
    );
    assert!(
        body_str.contains("questsCompleted"),
        "Response should patch questsCompleted signal"
    );

    println!("Completed quest glow class integration test completed successfully");
}

#[tokio::test]
async fn test_quest_page_references_static_files() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let app = Router::new()
        .route("/", get(handlers::quests))
        .with_state(AppState {
            db,
            bcast: broadcast::channel::<handlers::ServerMessage>(128).0,
        });

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("/style.css"),
        "HTML should reference the CSS file"
    );

    println!("Static files references integration test completed successfully");
}

#[tokio::test]
async fn test_responsive_glow_adaptation() {
    let css = get_css_content();

    assert!(
        css.contains("@media (max-width: 768px)"),
        "CSS should contain responsive media query"
    );
    assert!(
        css.contains("(prefers-reduced-motion: reduce)"),
        "CSS should respect reduced motion preference"
    );

    println!("Responsive glow adaptation integration test completed successfully");
}

#[tokio::test]
async fn test_oklch_color_variables_present() {
    let css = get_css_content();

    assert!(
        css.contains("--color-foreground"),
        "CSS should contain foreground color variable"
    );
    assert!(
        css.contains("--color-background"),
        "CSS should contain background color variable"
    );
    assert!(
        css.contains("--color-accent"),
        "CSS should contain accent color variable"
    );
    assert!(
        css.contains("--color-success"),
        "CSS should contain success color variable"
    );
    assert!(
        css.contains("--color-warning"),
        "CSS should contain warning color variable"
    );
    assert!(
        css.contains("--color-error"),
        "CSS should contain error color variable"
    );

    println!("OKLCH color variables integration test completed successfully");
}

#[tokio::test]
async fn test_pixel_unit_system_present() {
    let css = get_css_content();

    assert!(
        css.contains("--pixel-unit"),
        "CSS should contain pixel unit variable"
    );
    assert!(
        css.contains("--space-4px"),
        "CSS should contain 4px space variable"
    );
    assert!(
        css.contains("--space-8px"),
        "CSS should contain 8px space variable"
    );

    println!("Pixel unit system integration test completed successfully");
}

#[tokio::test]
async fn test_text_shadow_glow_present() {
    let css = get_css_content();

    assert!(
        css.contains("--text-glow-primary"),
        "CSS should contain primary text glow"
    );
    assert!(
        css.contains("--text-glow-success"),
        "CSS should contain success text glow"
    );
    assert!(
        css.contains("--text-glow-warning"),
        "CSS should contain warning text glow"
    );

    println!("Text shadow glow integration test completed successfully");
}

// ===== QUEST TOGGLE & ANIMATION INTEGRATION TESTS =====

fn get_js_content(path: &str) -> String {
    std::fs::read_to_string(path).expect(&format!("Failed to read JS file: {}", path))
}

#[tokio::test]
async fn test_counter_animation_js_exists() {
    // Counter animation was consolidated into app.js
    let js_path = std::path::Path::new("static/js/app.js");
    assert!(
        js_path.exists(),
        "App JS file should exist at {}",
        js_path.display()
    );

    let js_content = get_js_content("static/js/app.js");

    // Verify required functions exist (consolidated from counter-animation.js)
    assert!(
        js_content.contains("function pulseCounter"),
        "JS should contain pulseCounter function"
    );

    println!("Counter animation JS existence test completed successfully");
}

#[tokio::test]
async fn test_exp_pulse_animation_defined() {
    let css = get_css_content();

    assert!(
        css.contains("@keyframes exp-pulse"),
        "CSS should contain exp-pulse animation"
    );
    assert!(
        css.contains(".exp-pulse"),
        "CSS should contain exp-pulse class"
    );

    println!("EXP pulse animation integration test completed successfully");
}

#[tokio::test]
async fn test_celebration_particles_defined() {
    let css = get_css_content();

    assert!(
        css.contains(".celebration-particle"),
        "CSS should contain celebration-particle class"
    );
    assert!(
        css.contains("@keyframes particle-fly"),
        "CSS should contain particle-fly animation"
    );

    println!("Celebration particles integration test completed successfully");
}

#[tokio::test]
async fn test_view_transition_names_defined() {
    let css = get_css_content();

    assert!(
        css.contains("::view-transition-old(quest-morph)"),
        "CSS should contain view transition for quest-morph"
    );
    assert!(
        css.contains("::view-transition-old(exp-counter)"),
        "CSS should contain view transition for exp-counter"
    );
    assert!(
        css.contains("::view-transition-old(notification-new)"),
        "CSS should contain view transition for notifications"
    );

    println!("View transition names integration test completed successfully");
}

#[tokio::test]
async fn test_notification_animations_defined() {
    let css = get_css_content();

    assert!(
        css.contains(".notification-enter"),
        "CSS should contain notification-enter class"
    );
    assert!(
        css.contains(".notification-exit"),
        "CSS should contain notification-exit class"
    );
    assert!(
        css.contains("@keyframes notification-slide-in"),
        "CSS should contain notification-slide-in animation"
    );
    assert!(
        css.contains("@keyframes notification-slide-out"),
        "CSS should contain notification-slide-out animation"
    );

    println!("Notification animations integration test completed successfully");
}

// ===== MOBILE SUPPORT INTEGRATION TESTS =====

#[tokio::test]
async fn test_container_queries_defined() {
    let css = get_css_content();

    assert!(
        css.contains("container-type: inline-size"),
        "CSS should contain container-type for quest-item"
    );
    assert!(
        css.contains("container-name: quest-item"),
        "CSS should contain container-name for quest-item"
    );
    assert!(
        css.contains("@container quest-item"),
        "CSS should contain container query for quest-item"
    );

    println!("Container queries integration test completed successfully");
}

#[tokio::test]
async fn test_touch_target_sizes_defined() {
    let css = get_css_content();

    assert!(
        css.contains("min-height: 48px"),
        "CSS should contain 48px minimum touch target height"
    );
    assert!(
        css.contains("min-width: 120px"),
        "CSS should contain 120px minimum touch target width"
    );

    println!("Touch target sizes integration test completed successfully");
}

#[tokio::test]
async fn test_swipe_gesture_css_defined() {
    let css = get_css_content();

    assert!(
        css.contains(".quest-item::after"),
        "CSS should contain swipe indicator on quest-item"
    );
    assert!(
        css.contains("swipe-complete"),
        "CSS should contain swipe-complete animation"
    );
    assert!(
        css.contains("@keyframes swipe-complete"),
        "CSS should contain swipe-complete keyframes animation"
    );

    println!("Swipe gesture CSS integration test completed successfully");
}

#[tokio::test]
async fn test_landscape_mode_optimization_defined() {
    let css = get_css_content();

    assert!(
        css.contains("@media (orientation: landscape)"),
        "CSS should contain landscape orientation media query"
    );
    assert!(
        css.contains("grid-template-columns: repeat(auto-fill"),
        "CSS should contain grid layout for landscape mode"
    );

    println!("Landscape mode optimization integration test completed successfully");
}

#[tokio::test]
async fn test_high_dpi_optimization_defined() {
    let css = get_css_content();

    assert!(
        css.contains("@media (min-resolution: 2dppx)"),
        "CSS should contain high-DPI media query"
    );
    assert!(
        css.contains("@media (dynamic-range: high)"),
        "CSS should contain HDR display media query"
    );

    println!("High-DPI optimization integration test completed successfully");
}

#[tokio::test]
async fn test_thumb_friendly_layout_defined() {
    let css = get_css_content();

    assert!(
        css.contains("@media (pointer: coarse)"),
        "CSS should contain coarse pointer (touch) media query"
    );
    assert!(
        css.contains("position: fixed"),
        "CSS should contain fixed footer for mobile"
    );
    assert!(
        css.contains(".app-footer"),
        "CSS should contain app-footer styles"
    );

    println!("Thumb-friendly layout integration test completed successfully");
}

#[tokio::test]
async fn test_foldable_device_support_defined() {
    let css = get_css_content();

    assert!(
        css.contains("@media (spanning: single-fold-vertical)"),
        "CSS should contain foldable device spanning media query"
    );

    println!("Foldable device support integration test completed successfully");
}
