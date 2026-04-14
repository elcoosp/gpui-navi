//! User-related routes.

/// Users list page component.
pub struct UsersPage;

impl UsersPage {
    pub fn new() -> Self {
        Self
    }
}

/// User detail page component.
pub struct UserDetailPage {
    pub user_id: String,
}

impl UserDetailPage {
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
        }
    }
}
