use axum::{extract::{Path, State}, response::{IntoResponse, Redirect}, routing::get, Form, Router};
use axum_htmx::HxRequest;
use axum_login::login_required;
use chrono::Local;
use maud::{html, Markup};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::{auth::{AuthSession, AuthState}, backend::Backend, models::{InsertListing, Listing, ListingType}, AppState, LOGIN_URL};

mod components;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/listing/new", get(render_create_listing).post(create_listing))
        .route_layer(login_required!(AuthState, login_url = LOGIN_URL))
        .route("/listing/:id", get(show_listing).post(show_listing))
        .route("/home", get(render_homepage))
        .route("/about", get(render_about))
        .route("/discover", get(render_discover))
}

async fn show_listing(
    auth_session: AuthSession,
    HxRequest(is_htmx): HxRequest,
    State(backend): State<Backend>,
    Path(id): Path<i32>
) -> Markup {
    let listing = backend.get_listing(id).await;
    let content = match listing {
        Err(err) => {
            error!(id, ?err, "Error while getting listing");
            html! {
                span .center-page {
                    "Internal server error"
                }
            }
        }
        Ok(Some(listing)) => {
            html! {
                div #listing class={"text-white" (components::CARD)} {
                    h1 .text-2xl { (listing.title) }
                    p { (listing.description) }
                    (render_listing_insertion_date(&listing))
                }
            }
        },
        Ok(None) => html! { span .center-page { "404: Couldn't find listing" } }
    };

    render_page(None, content, is_htmx, auth_session)
}

fn render_listing_insertion_date(listing: &Listing) -> Markup {
    let now = Local::now().naive_local();
    let duration = -listing.insertion_date.signed_duration_since(now);

    let insertion_date = listing.insertion_date.format("%H:%M %Y.%m.%d");

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

    html! {
        p .tooltip.text-slate-400.text-right {
            (human_duration)
            span .tooltiptext { (insertion_date) }
        }
    }
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
            render_page(None, html!{ "Internal server error, try again later" }, true, auth_session).into_response()
        }
    }
}

async fn render_create_listing(HxRequest(is_htmx): HxRequest, auth_session: AuthSession) -> Markup {
    let content = html! {
        form #new-listing class=(components::CARD) hx-post="/listing/new" hx-target="#page" hx-swap="outerHTML"
            action="/listing/new"
        {
            h5 class="mb-2 text-2xl font-bold tracking-tight text-gray-900 dark:text-white" {
                "Insert new listing"
            }

            (components::text_input("title", "Title", "Your title here", true))

            (components::textarea_input(
                "description",
                "Description",
                "Describe your plant",
                true)
            )

            .py-4 {
                (components::radio("listing_type", [
                    ("Selling", true),
                    ("Buying", false),
                    ])
                )
            }

            (components::checkbox("tradeable", "Trade possible"))

            button type="submit" form="new-listing"
                class={"self-end" (components::button::GREEN)} {
                    "Create listing"
                }
        }
    };

    render_page(None, content, is_htmx, auth_session)
}

async fn render_discover(State(backend): State<Backend>, auth_session: AuthSession, HxRequest(is_htmx): HxRequest) -> Markup {
    let listings = match backend.get_all_listings().await {
        Err(err) => {
            error!(?err, "Discover page failed");
            let content = html! { span .center-page { "Internal server error" } };
            return render_page(Some(PageSelection::Discover), content, is_htmx, auth_session);
        },
        Ok(listings) => listings
    };

    let content = html! {
        div .w-full.flex.flex-col.items-center.gap-2 {
            @for listing in listings {
                a href={"/listing/" (listing.id)} hx-get={"/listing/" (listing.id)}
                    hx-replace-url="true" hx-target="#page" hx-swap="outerHTML"
                    class={"flex flex-col gap-2 p-6 w-3/4" (components::CARD)} {
                    h1 .text-2xl { (listing.title) }
                    p .p-2.border.border-gray-300.rounded-lg { (listing.description) }
                    p {
                        "Tradeable: "
                        b { @if listing.tradeable { "Yes" } @else { "No" } }
                    }
                    .self-end { (render_listing_insertion_date(&listing)) }
                }
            }
        }
    };
    render_page(Some(PageSelection::Discover), content, is_htmx, auth_session)
}

async fn render_homepage(HxRequest(is_htmx): HxRequest, auth_session: AuthSession) -> Markup {
    let content = html! {
        p .center-page {
            "A place for you to buy, sell, trade, and gift plants to other people."
        }
    };

    render_page(Some(PageSelection::Home), content, is_htmx, auth_session)
}

async fn render_about(HxRequest(is_htmx): HxRequest, auth_session: AuthSession) -> Markup {
    let content = html! {
        div .center-page.flex.flex-col.items-center.gap-2 {
            p { "A place for you to buy, sell, trade, and gift plants to other people." }
            p { "Made by Emilia Jaser." }
            p { "Proudly free and open source." }
        }
    };

    render_page(Some(PageSelection::About), content, is_htmx, auth_session)
}

fn render_page(page_selection: Option<PageSelection>, content: Markup, is_htmx: bool, auth_session: AuthSession) -> Markup {
    if is_htmx {
        html! {
            (render_page_selector(page_selection))
            article #page { (content) }
        }
    } else {
        html! {
            (maud::DOCTYPE)
            html {
                (render_head())
                body {
                    div #main-wrapper {
                        header { (render_header(page_selection, auth_session))}
                        article #page { (content) }
                        footer { "Footer" }
                    }
                }
            }
        }
    }
}

fn render_header(page_selection: Option<PageSelection>, auth_session: AuthSession) -> Markup {
    html! {
        div .header-left-side {
            div .header-site-name {
                "Plant swap"
            }
            (render_page_selector(page_selection))
        }
        (render_login_button(auth_session))
    }
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

fn render_page_selector(current_selection: Option<PageSelection>) -> Markup {
    html! {
        nav #nav-selector hx-swap-oob="true" hx-replace-url="true" hx-target="#page" hx-swap="outerHTML" {
            @for (selection, display_name, href) in PAGE_SELECTIONS {
                a href=(href) hx-get=(href) .page-selector
                    .current-page-selector[selection.is_some_and(|s| Some(s) == current_selection)] {
                    (display_name)
                }
            }
        }
    }
}

fn render_login_button(auth_session: AuthSession) -> Markup {
    html! {
        div .login-or-profile-button {
            @if let Some(user) = auth_session.user {
                (user.claims.name)
            } @else {
                a href="/auth/login" { "Login" }
            }
        }
    }
}

fn render_head() -> Markup {
    html! {
        head {
            title { "Plant swap" }
            meta name="title" content="Plant swap";
            meta charset="UTF-8";
            meta name="description" content="A place for people to buy/sell/trade plants";
            meta name="author" content="Emilia Jaser";
            link rel="stylesheet" "type"="text/css" href="/assets/styles.css";
            script src="/assets/script/htmx-2.0.2.js" { }
            script src="/assets/script/tailwind-3.4.5.js" { }
        }
    }
}
