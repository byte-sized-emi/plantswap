use askama::DynTemplate;
use axum::{
    extract::{Path, State}, http::StatusCode, response::{IntoResponse, Redirect}, routing::get, Router
};
use axum_htmx::HxRequest;
use axum_login::login_required;
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use itertools::Itertools;
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::{AuthSession, AuthState},
    backend::{Backend, BackendError},
    models::{InsertListing, ListingType},
    AppState, LOGIN_URL,
};

mod templates;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/listing/new",
            get(render_create_listing).post(create_listing),
        )
        .route_layer(login_required!(AuthState, login_url = LOGIN_URL))
        .route(
            "/listing/:humanname/:id",
            get(show_listing).post(show_listing),
        )
        .route("/home", get(render_homepage))
        .route("/about", get(render_about))
        .route("/discover", get(render_discover))
}

pub async fn fallback_handler(
    HxRequest(is_htmx): HxRequest,
    auth_session: AuthSession,
) -> (StatusCode, impl IntoResponse) {
    let page = templates::pages::Error404Page;
    let rendered_page = render_htmx_page(is_htmx, None, auth_session, Box::new(page));

    (StatusCode::NOT_FOUND, rendered_page)
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
    page: Box<dyn DynTemplate>,
) -> impl IntoResponse {
    if is_htmx {
        templates::PageReplacement {
            page_selector: templates::PageSelector { current_selection },
            page,
        }
        .into_response()
    } else {
        templates::Base {
            page_selector: templates::PageSelector { current_selection },
            login_button: templates::LoginButton { auth_session },
            page,
        }
        .into_response()
    }
}

async fn show_listing(
    auth_session: AuthSession,
    HxRequest(is_htmx): HxRequest,
    State(backend): State<Backend>,
    Path((_human_name, id)): Path<(String, Uuid)>,
) -> impl IntoResponse {
    let listing = backend.get_listing(id).await;
    let content: Box<dyn DynTemplate> = match listing {
        Err(err) => {
            error!(?id, ?err, "Error while getting listing");
            Box::new(templates::pages::Error::new("Internal server error"))
        }
        Ok(Some(listing)) => Box::new(templates::pages::ShowListing { listing }),
        Ok(None) => Box::new(templates::pages::Error::new("404 Couldn't find listing")),
    };

    render_htmx_page(is_htmx, None, auth_session, content)
}

#[derive(TryFromMultipart)]
struct InsertListingBody {
    pub title: String,
    #[form_data(default)]
    pub description: String,
    pub listing_type: ListingType,
    #[form_data(limit = "10MiB")]
    pub pictures: Vec<FieldData<axum::body::Bytes>>,
    #[form_data(default)]
    pub tradeable: bool,
}

impl InsertListingBody {
    pub fn into_insert_listing(self, author: Uuid, thumbnail: Uuid) -> InsertListing {
        InsertListing {
            title: self.title,
            description: self.description,
            author,
            listing_type: self.listing_type,
            tradeable: Some(self.tradeable),
            thumbnail,
        }
    }
}

async fn create_listing(
    auth_session: AuthSession,
    State(backend): State<Backend>,
    TypedMultipart(body): TypedMultipart<InsertListingBody>,
) -> impl IntoResponse {
    let author = auth_session.user.as_ref().unwrap().claims.user_id;

    if body.pictures.is_empty() {
        let page = templates::pages::CreateListing::with_error("You need to upload at least one image");
        return render_htmx_page(true, None, auth_session, Box::new(page)).into_response();
    }

    for picture in &body.pictures {
        let content_type = picture.metadata.content_type
            .as_ref()
            .map(|f| f.as_ref());
        if content_type != Some("image/jpeg") && content_type != Some("image/png") {
            error!(?content_type, "Invalid content type");
            let page = templates::pages::CreateListing::with_error("Invalid image type");
            return render_htmx_page(true, None, auth_session, Box::new(page)).into_response();
        }
    }

    let user_id = auth_session.user.as_ref().unwrap().claims.user_id;

    let mut picture_ids = Vec::new();

    for picture in &body.pictures {
        let upload_result = backend
            .upload_image(user_id, picture.contents.clone())
            .await;
        match upload_result {
            Ok(uuid) => picture_ids.push(uuid),
            Err(err) => {
                error!(?err, "Error while uploading image");
                let page = templates::pages::CreateListing::with_error("Internal server error");
                return render_htmx_page(true, None, auth_session, Box::new(page)).into_response();
            }
        }
    }

    let thumbnail = picture_ids.first().unwrap();

    let mut insert_listing = body.into_insert_listing(author, *thumbnail);

    // Cleanup title
    insert_listing.title = insert_listing
        .title
        .split_whitespace()
        .map(|word| word.trim())
        .join(" ");

    match backend.create_listing(insert_listing).await {
        Ok(listing) => {
            let id = listing.id;

            let human_name = convert_title_to_human_url(listing.title);

            Redirect::permanent(&format!("/listing/{human_name}/{id}")).into_response()
        }
        Err(BackendError::ListingHasNoLocation) => {
            let page = templates::pages::CreateListing::with_error(
                "Your account needs to have a location set in order to create a listing"
            );
            render_htmx_page(true, None, auth_session, Box::new(page)).into_response()
        }
        Err(err) => {
            error!(?err, "Database error while creating listing");
            let page = templates::pages::CreateListing::with_error("Internal server error, try again later");
            render_htmx_page(true, None, auth_session, Box::new(page)).into_response()
        }
    }
}

fn convert_title_to_human_url(title: String) -> String {
    return title
        .chars()
        .flat_map(|c| c.to_lowercase())
        .flat_map(|c| match c {
            c if c.is_ascii_alphanumeric() => vec![c],
            c if c.is_whitespace() => vec!['-'],
            'ä' => vec!['a', 'e'],
            'ö' => vec!['o', 'e'],
            'ü' => vec!['u', 'e'],
            'ß' => vec!['s'],
            _ => vec![],
        })
        .take(80)
        .collect();
}

async fn render_create_listing(
    HxRequest(is_htmx): HxRequest,
    auth_session: AuthSession,
) -> impl IntoResponse {
    render_htmx_page(
        is_htmx,
        None,
        auth_session,
        Box::new(templates::pages::CreateListing::new()),
    )
}

async fn render_discover(
    State(backend): State<Backend>,
    auth_session: AuthSession,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    let listings = match backend.get_all_listings().await {
        Err(err) => {
            error!(?err, "Discover page failed");
            let page = templates::pages::Error::new("Internal server error");
            return render_htmx_page(
                is_htmx,
                Some(PageSelection::Discover),
                auth_session,
                Box::new(page),
            )
            .into_response();
        }
        Ok(listings) => listings,
    };

    let page = templates::pages::Discover { listings };
    render_htmx_page(
        is_htmx,
        Some(PageSelection::Discover),
        auth_session,
        Box::new(page),
    )
    .into_response()
}

async fn render_homepage(
    HxRequest(is_htmx): HxRequest,
    auth_session: AuthSession,
) -> impl IntoResponse {
    let page = templates::pages::Home;

    render_htmx_page(
        is_htmx,
        Some(PageSelection::Home),
        auth_session,
        Box::new(page),
    )
}

async fn render_about(
    HxRequest(is_htmx): HxRequest,
    auth_session: AuthSession,
) -> impl IntoResponse {
    let page = templates::pages::About;

    render_htmx_page(
        is_htmx,
        Some(PageSelection::About),
        auth_session,
        Box::new(page),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PageSelection {
    Home,
    About,
    Discover,
}

/// PageSelection for displaying current page, display name, href
const PAGE_SELECTIONS: &[(Option<PageSelection>, &str, &str)] = &[
    (Some(PageSelection::Home), "Home", "/home"),
    (Some(PageSelection::About), "About", "/about"),
    (Some(PageSelection::Discover), "Discover", "/discover"),
    (None, "Create listing", "/listing/new"),
];
