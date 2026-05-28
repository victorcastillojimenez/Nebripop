// AuthUser extractor is defined in common crate (`common::auth::AuthUser`)
// and implements FromRequestParts via `String: FromRef<S>`.
//
// Sub-crates (ratings, favorites, geo) use it directly as:
//   use common::auth::AuthUser;
//   async fn handler(auth_user: AuthUser) -> ...
//
// The FromRef<S> impl for String is provided in app_state.rs.
