use askama::DynTemplate;
use axum::{extract::{Path, State}, response::{IntoResponse, Redirect}, routing::get, Form, Router};
use axum_htmx::HxRequest;
use axum_login::login_required;
use chrono::{Local, NaiveDateTime};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::{auth::{AuthSession, AuthState}, backend::Backend, models::{InsertListing, ListingType}, AppState, LOGIN_URL};

mod templates;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/listing/new", get(render_create_listing).post(create_listing))
        .route_layer(login_required!(AuthState, login_url = LOGIN_URL))
        .route("/listing/:id", get(show_listing).post(show_listing))
        .route("/home", get(render_homepage))
        .route("/about", get(render_about))
        .route("/discover", get(render_discover))
}

mod components {
    #[allow(unused)]
    pub mod button {
        pub const DEFAULT: &str = " text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800 ";
        pub const ALTERNATIVE: &str = " py-2.5 px-5 me-2 mb-2 text-sm font-medium text-gray-900 focus:outline-none bg-white rounded-lg border border-gray-200 hover:bg-gray-100 hover:text-blue-700 focus:z-10 focus:ring-4 focus:ring-gray-100 dark:focus:ring-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-600 dark:hover:text-white dark:hover:bg-gray-700 ";
        pub const DARK: &str = " text-white bg-gray-800 hover:bg-gray-900 focus:outline-none focus:ring-4 focus:ring-gray-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-gray-800 dark:hover:bg-gray-700 dark:focus:ring-gray-700 dark:border-gray-700 ";
        pub const LIGHT: &str = " text-gray-900 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:hover:border-gray-600 dark:focus:ring-gray-700 ";
        pub const GREEN: &str = " focus:outline-none text-white bg-green-700 hover:bg-green-800 focus:ring-4 focus:ring-green-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-green-600 dark:hover:bg-green-700 dark:focus:ring-green-800 ";
        pub const RED: &str = " focus:outline-none text-white bg-red-700 hover:bg-red-800 focus:ring-4 focus:ring-red-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-red-600 dark:hover:bg-red-700 dark:focus:ring-red-900";
        pub const YELLOW: &str = " focus:outline-none text-white bg-yellow-400 hover:bg-yellow-500 focus:ring-4 focus:ring-yellow-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:focus:ring-yellow-900 ";
        pub const PURPLE: &str = " focus:outline-none text-white bg-purple-700 hover:bg-purple-800 focus:ring-4 focus:ring-purple-300 font-medium rounded-lg text-sm px-5 py-2.5 mb-2 dark:bg-purple-600 dark:hover:bg-purple-700 dark:focus:ring-purple-900 ";
    }

    pub const CARD: &str = "
        block bg-white border border-gray-200 rounded-lg shadow
        text-white
        dark:bg-gray-800 dark:border-gray-700
    ";
}

fn render_htmx_page(
    is_htmx: bool,
    current_selection: Option<PageSelection>,
    auth_session: AuthSession,
    page: Box<dyn DynTemplate>
) -> impl IntoResponse {
    if is_htmx {
        templates::PageReplacement {
            page_selector: templates::PageSelector {
                current_selection
            },
            page
        }
        .into_response()
    } else {
        templates::Base {
            page_selector: templates::PageSelector {
                current_selection
            },
            login_button: templates::LoginButton { auth_session },
            page
        }
        .into_response()
    }
}

async fn show_listing(
    auth_session: AuthSession,
    HxRequest(is_htmx): HxRequest,
    State(backend): State<Backend>,
    Path(id): Path<i32>
) -> impl IntoResponse {
    let listing = backend.get_listing(id).await;
    let content: Box<dyn DynTemplate> = match listing {
        Err(err) => {
            error!(id, ?err, "Error while getting listing");
            Box::new(templates::pages::Error::new("Internal server error"))
        }
        Ok(Some(listing)) => {
            Box::new(templates::pages::ShowListing { listing })
        },
        Ok(None) => Box::new(templates::pages::Error::new("404 Couldn't find listing"))
    };

    render_htmx_page(is_htmx, None, auth_session, content)
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

#[derive(Deserialize)]
struct InsertListingBody {
    pub title: String,
    pub description: String,
    pub listing_type: ListingType,
    #[serde(default)]
    pub tradeable: bool,
}

impl InsertListingBody {
    pub fn to_insert_listing(self, author: Uuid) -> InsertListing {
        InsertListing {
            title: self.title,
            description: self.description,
            author,
            listing_type: self.listing_type,
            tradeable: Some(self.tradeable)
        }
    }
}

async fn create_listing(
    auth_session: AuthSession,
    State(backend): State<Backend>,
    Form(body): Form<InsertListingBody>
) -> impl IntoResponse {
    let author = auth_session.user.as_ref().unwrap().claims.user_id.clone();
    let insert_listing = body.to_insert_listing(author);

    match backend.create_listing(insert_listing).await {
        Ok(listing) => Redirect::permanent(&format!("/listing/{}", listing.id)).into_response(),
        Err(err) => {
            error!(?err, "Database error while creating listing");
            let page = templates::pages::Error::new("Internal server error, try again later");
            render_htmx_page(true, None, auth_session, Box::new(page)).into_response()
        }
    }
}

async fn render_create_listing(HxRequest(is_htmx): HxRequest, auth_session: AuthSession)
-> impl IntoResponse {
    render_htmx_page(
        is_htmx,
        None,
        auth_session,
        Box::new(templates::pages::CreateListing)
    )
}

async fn render_discover(State(backend): State<Backend>, auth_session: AuthSession, HxRequest(is_htmx): HxRequest) -> impl IntoResponse {
    let listings = match backend.get_all_listings().await {
        Err(err) => {
            error!(?err, "Discover page failed");
            let page = templates::pages::Error::new("Internal server error");
            return render_htmx_page(is_htmx, Some(PageSelection::Discover), auth_session, Box::new(page))
                .into_response();
        },
        Ok(listings) => listings
    };

    let page = templates::pages::Discover { listings };
    render_htmx_page(is_htmx, Some(PageSelection::Discover), auth_session, Box::new(page))
        .into_response()
}

async fn render_homepage(HxRequest(is_htmx): HxRequest, auth_session: AuthSession) -> impl IntoResponse {
    let page = templates::pages::Home;

    render_htmx_page(is_htmx, Some(PageSelection::Home), auth_session, Box::new(page))
}

async fn render_about(HxRequest(is_htmx): HxRequest, auth_session: AuthSession) -> impl IntoResponse {
    let page = templates::pages::About;

    render_htmx_page(is_htmx, Some(PageSelection::About), auth_session, Box::new(page))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PageSelection {
    Home,
    About,
    Discover
}

/// PageSelection for displaying current page, display name, href
const PAGE_SELECTIONS: &[(Option<PageSelection>, &str, &str)] = &[
    (Some(PageSelection::Home), "Home", "/home"),
    (Some(PageSelection::About), "About", "/about"),
    (Some(PageSelection::Discover), "Discover", "/discover"),
    (None, "Create listing", "/listing/new")
];
