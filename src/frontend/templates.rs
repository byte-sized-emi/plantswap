use askama::DynTemplate;
use askama_axum::Template;
use chrono::{Local, NaiveDateTime};

use crate::auth::AuthSession;

use super::PageSelection;

#[derive(Template)]
#[template(path = "page_selector.html")]
pub struct PageSelector {
    pub current_selection: Option<PageSelection>,
}

#[derive(Template)]
#[template(path = "base.html")]
pub struct Base {
    pub page_selector: PageSelector,
    pub login_button: LoginButton,
    pub page: Box<dyn DynTemplate>
}

#[derive(Template)]
#[template(source = r#"
    {{ page_selector|safe }}
    <article id="page">{{ page|safe }}</article>
"#, ext = "txt")]
pub struct PageReplacement {
    pub page_selector: PageSelector,
    pub page: Box<dyn DynTemplate>
}

#[derive(Template)]
#[template(path = "login_button.html")]
pub struct LoginButton {
    pub auth_session: AuthSession
}

pub mod pages {
    use askama_axum::Template;

    use crate::frontend::components;
    use super::generate_insertion_date;
    pub use crate::models::Listing;

    #[derive(Template)]
    #[template(path = "pages/about.html")]
    pub struct About;

    #[derive(Template)]
    #[template(path = "pages/home.html")]
    pub struct Home;

    #[derive(Template)]
    #[template(path = "pages/discover.html")]
    pub struct Discover {
        pub listings: Vec<Listing>,
    }

    #[derive(Template)]
    #[template(path = "pages/show_listing.html")]
    pub struct ShowListing {
        pub listing: Listing
    }

    #[derive(Template)]
    #[template(path = "pages/create_listing.html")]
    pub struct CreateListing<'a> {
        pub error: Option<&'a str>,
    }

    impl<'a> CreateListing<'a> {
        pub fn new() -> Self {
            Self { error: None }
        }

        pub fn with_error(error: &'a str) -> Self {
            Self { error: Some(error) }
        }
    }

    #[derive(Template)]
    #[template(source = "<span class=\"center-page\">{{ error }}</span>", ext = "txt")]
    pub struct Error<'a> {
        pub error: &'a str
    }

    impl<'a> Error<'a> {
        pub fn new(error: &'a str) -> Self {
            Error { error }
        }
    }
}

fn generate_insertion_date(insertion_date: &NaiveDateTime) -> (String, String) {
    let now = Local::now().naive_local();
    let duration = -insertion_date.signed_duration_since(now);

    let insertion_date = insertion_date.format("%H:%M %Y.%m.%d");

    let human_duration = match duration {
        dur if dur.num_days() > 365 => {
            format!("{} years ago", dur.num_days() / 365)
        }
        dur if dur.num_days() > 1 => {
            format!("{} days ago", dur.num_days())
        }
        dur if dur.num_hours() > 1 => {
            format!("{} hours ago", dur.num_hours())
        }
        dur if dur.num_minutes() > 15 => {
            format!("{} minutes ago", dur.num_minutes())
        }
        _ => {
            "Just now".to_string()
        }
    };

    (human_duration, insertion_date.to_string())
}

fn is_current_selection(selection: &Option<PageSelection>, current_selection: &Option<PageSelection>) -> bool {
    selection.is_some_and(|s| &Some(s) == current_selection)
}
