## Features

- **User Authentication & Authorization**: Secure user registration and login using JWT tokens. Middleware ensures protected routes are accessed only by authenticated users.
- **Content Access Control**: Enforces access rules based on user subscription plans (e.g., Free, Basic, Premium).
- **Machine Learning Integration**: Uses a simple ML model (Decision Tree) to analyze user behavior and make dynamic access decisions for users without sufficient subscriptions.
- **Database Persistence**: Stores user data, subscriptions, content, and user interaction logs using PostgreSQL via `sqlx`.
- **Caching**: Employs in-memory caching (`moka`) to improve performance for frequently requested content access checks.
- **Logging**: Comprehensive logging using `tracing` for monitoring and debugging.
- **Configuration Management**: Securely loads configuration (database URL, secrets) from environment variables using `dotenv` and `envy`.
- **Simulated Payment Processing**: Includes a placeholder for integrating with real payment gateways.

## Architecture Overview

The application follows a layered architecture:

1.  **API Layer (`main.rs`, `auth.rs`, `paywall.rs`)**: Handles HTTP requests/responses using Actix Web. Defines routes and orchestrates calls to business logic.
2.  **Business Logic Layer (`auth.rs`, `paywall.rs`, `ml.rs`)**: Contains the core logic for authentication, content access decisions (including ML), and subscription management.
3.  **Data Access Layer (`db.rs`)**: Abstracts database interactions using `sqlx`. Provides functions to query and update user, subscription, content, and behavior data.
4.  **ML Layer (`ml.rs`)**: Encapsulates the machine learning model and feature extraction logic.
5.  **Configuration (`config.rs`)**: Manages application settings.
6.  **Models (`models.rs`)**: Defines shared data structures (DTOs) used across the application.

Middleware (`auth.rs`) intercepts requests to protected routes, validates JWT tokens, and injects user identity into the request context.

## Technology Stack

- **Language**: [Rust](https://www.rust-lang.org/)
- **Web Framework**: [Actix Web](https://actix.rs/)
- **Database**: [PostgreSQL](https://www.postgresql.org/)
- **Database Driver**: [SQLx](https://github.com/launchbadge/sqlx) (Compile-time checked queries)
- **Authentication**: [JWT](https://crates.io/crates/jsonwebtoken), [Bcrypt](https://crates.io/crates/bcrypt)
- **Caching**: [Moka](https://crates.io/crates/moka) (Async cache)
- **Machine Learning**: [Linfa](https://crates.io/crates/linfa), [Ndarray](https://crates.io/crates/ndarray) (Decision Trees)
- **Logging**: [Tracing](https://crates.io/crates/tracing), [Tracing Subscriber](https://crates.io/crates/tracing-subscriber)
- **Configuration**: [Dotenv](https://crates.io/crates/dotenv), [Envy](https://crates.io/crates/envy)
- **Serialization**: [Serde](https://crates.io/crates/serde), [Serde JSON](https://crates.io/crates/serde_json)
- **UUIDs**: [Uuid](https://crates.io/crates/uuid)
- **Date/Time**: [Chrono](https://crates.io/crates/chrono)
- **Async Runtime**: [Tokio](https://tokio.rs/)

## Prerequisites

- **Rust Toolchain**: Install via [rustup](https://rustup.rs/). Ensure you have Rust 1.70+ (check with `rustc --version`).
- **PostgreSQL Database**: Install and run a PostgreSQL server (version 12+ recommended). You can use Docker, a local installation, or a cloud provider.
- **Git**: For cloning the repository.

## Project Setup

### 1. Clone the Repository

```bash
git clone https://github.com/your-username/advanced-paywall-rust.git
cd advanced-paywall-rust

# Database connection string
DATABASE_URL=postgres://username:password@localhost:5432/advanced_paywall_db

# Secret key for signing JWT tokens. Use a long, random, secret string.
# Generate one with: openssl rand -base64 32
JWT_SECRET=your_very_long_and_secret_jwt_signing_key_here

# (Placeholder) API key for a real payment provider (e.g., Stripe)
PAYMENT_API_KEY=your_real_payment_provider_api_key

# (Placeholder) URL for a real payment provider's API endpoint
PAYMENT_API_URL=https://api.yourpaymentprovider.com/charge

# Logging level (trace, debug, info, warn, error)
RUST_LOG=info

# Optional: Specify the port if different from default (8080)
# PORT=8080

API Endpoints
Authentication

    POST /auth/register
        Registers a new user.
        Request Body: { "username": "...", "email": "...", "password": "..." }
        Response:
            201 Created: { "message": "User created successfully", "user_id": "..." }
            409 Conflict: { "error": "Username already exists" }
            500 Internal Server Error: { "error": "Internal server error" }



    POST /auth/login
        Authenticates a user and issues a JWT.
        Request Body: { "username": "...", "password": "..." }
        Response:
            200 OK: { "token": "JWT_TOKEN_HERE", "user_id": "..." }
            401 Unauthorized: { "error": "Invalid credentials" }
            500 Internal Server Error: { "error": "Internal server error" }




Paywall

    GET /content/{content_id} (Requires Authentication)
        Attempts to retrieve content based on the user's subscription.
        Headers: Authorization: Bearer JWT_TOKEN_HERE
        Response:
            200 OK:
                If access granted: { "content": { ...content data... }, "access_granted": true }
                If access denied but ML suggests action: { "content": { ... }, "access_granted": false, "ml_suggestion": "..." }
                If access denied: { "content": { ... }, "access_granted": false, "message": "Upgrade..." }

            401 Unauthorized: { "error": "Unauthorized" }
            404 Not Found: { "error": "Content not found" }
            500 Internal Server Error: { "error": "Internal server error" }



    POST /subscription/purchase (Requires Authentication)
        Simulates purchasing a subscription plan.
        Headers: Authorization: Bearer JWT_TOKEN_HERE
        Request Body: { "plan_id": "basic|premium", "payment_token": "..." } (Payment token is a placeholder)
        Response:
            200 OK: { "message": "Subscription purchased successfully", "subscription_id": "...", "expires_at": "..." }
            400 Bad Request: { "error": "Invalid plan" }
            401 Unauthorized: { "error": "Unauthorized" }
            402 Payment Required: { "error": "Payment failed" } (Simulated)
            500 Internal Server Error: { "error": "Internal server error" | "Payment processing error" }



    GET /user/profile (Requires Authentication)
        Retrieves the authenticated user's profile information, including subscription status and behavior stats.
        Headers: Authorization: Bearer JWT_TOKEN_HERE
        Response:
            200 OK: { "user_id": "...", "subscription": { ... } | null, "total_interactions": ..., "avg_interaction_score": ... }
            401 Unauthorized: { "error": "Unauthorized" }
            500 Internal Server Error: { "error": "Internal server error" }




Core Components Explained
Authentication & Authorization

    auth.rs:
        Contains login and register endpoint handlers.
        Implements jwt_middleware using actix_web::middleware::from_fn. This middleware runs before protected routes.
            It extracts the Authorization: Bearer <token> header.
            It uses jsonwebtoken to decode and validate the token against the JWT_SECRET.
            If valid, it extracts the user_id from the token's claims and stores it in the request's extensions (req.extensions_mut().insert(...)).
            If invalid or missing, it returns a 401 Unauthorized response.

        get_user_id_from_request is a helper function used by other handlers to retrieve the authenticated user's ID from the request extensions.



Database Layer (db.rs)

    Uses sqlx for asynchronous, type-safe database queries.
    Functions like get_user_by_username, create_user, get_content_by_id, etc., encapsulate specific database operations.
    Leverages #[derive(sqlx::FromRow)] on structs in models.rs to automatically map query results (SELECT lists must match struct fields).
    Handles sqlx::Error internally or passes them up for the API layer to convert into HTTP responses.
    Includes helper functions for extracting ML features (get_user_avg_view_time, etc.) by querying aggregated data from user_behaviors.


Machine Learning (ml.rs)

    Model: Uses linfa_trees::DecisionTree<f32, f32> for binary classification (grant access: 1.0, deny: 0.0).
    Training: Currently uses generate_synthetic_data to create mock training data based on simple rules (e.g., users with high view time are likely to convert). In a real scenario, initialize_model would load data from the user_behaviors and subscriptions tables.
    Prediction: The predict method takes MLFeatures, converts them to a format suitable for the model (ndarray::Array2), and returns a boolean based on the model's output probability.
    Feature Extraction: extract_features aggregates user and content metrics from the database (db.rs helpers) to form the input vector for the model.


Caching

    Implemented using moka::future::Cache<String, serde_json::Value>.
    In paywall.rs, the result of content access checks (get_content) is cached using a key like content_<id>_user_<id>.
    This avoids re-computing access logic (including potential DB calls and ML prediction) for the same user/content combination within the cache's TTL (implicitly managed by Moka's eviction policy if configured, or until manual eviction).


Logging

    Uses the tracing crate for structured logging.
    tracing_subscriber configured in main.rs to output logs based on the RUST_LOG environment variable.
    Logs are emitted throughout the application for significant events (startup, requests, errors, cache hits, ML predictions) using macros like tracing::info!, tracing::warn!, tracing::error!.


Configuration

    config.rs defines a Config struct.
    dotenv::dotenv() loads variables from the .env file into the process environment.
    envy::from_env::<Config>() automatically maps environment variables to the Config struct fields based on their names.


Security Considerations

    JWT Secret: The JWT_SECRET must be kept absolutely secret. Use a strong, randomly generated key. Never hardcode it or commit it.
    Password Hashing: Passwords are hashed using bcrypt before storage.
    SQL Injection: sqlx with prepared statements ($1, $2) prevents SQL injection.
    Authentication Middleware: Centralized JWT validation ensures only authenticated users access protected endpoints.
    Error Handling: Generic error messages are returned to the client to avoid leaking internal details. Detailed errors are logged server-side.


Extending the System

    Real Payment Integration: Replace the process_payment stub in paywall.rs with actual calls to a payment provider's API (e.g., Stripe).
    Advanced ML Models: Integrate more sophisticated models (e.g., using ONNX Runtime, TensorFlow Serving) or train models externally and load them.
    Subscription Tiers: Add more complex subscription logic (e.g., trial periods, family plans).
    Content Types: Extend the content table and logic to handle different content types (videos, images, documents).
    Analytics Dashboard: Build an API/UI to visualize user behavior data from user_behaviors.
    Caching Strategy: Configure explicit TTLs, cache warming strategies, or use Redis for distributed caching.
    Background Jobs: Use a worker queue (e.g., faktory) for tasks like sending emails, processing analytics data, or retraining ML models.


Troubleshooting

    Compilation Errors: Ensure all dependencies in Cargo.toml are correctly listed and compatible versions are used. Run cargo clean and cargo build if issues persist.
    Database Connection Failed: Verify the DATABASE_URL in .env is correct (username, password, host, port, database name). Ensure the PostgreSQL server is running and accessible.
    "Invalid token" Errors: Check that the JWT_SECRET in .env matches the one used to sign the token. Ensure the token hasn't expired (default 24h).
    ML Model Initialization Errors: Check the synthetic data generation logic or the data loading logic if connecting to real data.
    Permission Denied (Database): Ensure the database user specified in DATABASE_URL has the necessary privileges (SELECT, INSERT, UPDATE, DELETE) on the tables.


Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues to discuss improvements, bug fixes, or new features. Ensure code adheres to Rust best practices and formatting (cargo fmt, cargo clippy).
License

This project is licensed under the MIT License . (You should include a LICENSE file in your repository if you choose a specific license).
