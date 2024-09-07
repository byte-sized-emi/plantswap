use axum::{routing::get, Router};
use axum_htmx::HxRequest;
use axum_login::login_required;
use maud::{html, Markup};

use crate::{auth::{AuthSession, AuthState}, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/listing/new", get(render_create_listing))
        .route_layer(login_required!(AuthState, login_url = "/auth/login"))
        .route("/home", get(render_homepage))
        .route("/about", get(render_about))
}

async fn render_create_listing(HxRequest(is_htmx): HxRequest, auth_session: AuthSession) -> Markup {
    let content = html! {
        form .new-listing-form hx-post="/listing/new" hx-target="#page" hx-swap="outerHTML"
            action="/listing/new"
        {
            label for="title" { "Title" }
            input #title type="text" name="title" { }

            label for="description" { "Description" }
            textarea #description name="description" { }

            // listing_type
            label for="buying" { "Buying" }
            input #buying type="radio" name="listing_type" value="buying" { }

            label for="selling" { "Selling" }
            input #selling type="radio" name="listing_type" value="selling" { }
            // tradeable
            label for="tradeable" { "Trade possible"}
            input #tradeable type="checkbox" name="tradeable" { }

            input type="submit" { "Create listing" }
        }
    };

    render_page(Some(PageSelection::About), content, is_htmx, auth_session)
}

async fn render_homepage(HxRequest(is_htmx): HxRequest, auth_session: AuthSession) -> Markup {
    let content = html! {
        p {
            "A place for you to buy, sell, trade, and gift plants to other people."
        }
    };

    render_page(Some(PageSelection::Home), content, is_htmx, auth_session)
}

async fn render_about(HxRequest(is_htmx): HxRequest, auth_session: AuthSession) -> Markup {
    let content = html! {
        p {
            "A place for you to buy, sell, trade, and gift plants to other people."
            "Made by Emilia Jaser."
            "Proudly free and open source."
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
}

/// PageSelection for displaying current page, display name, href
const PAGE_SELECTIONS: &[(Option<PageSelection>, &str, &str)] = &[
    (Some(PageSelection::Home), "Home", "/home"),
    (Some(PageSelection::About), "About", "/about"),
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
            script src="/assets/script/htmx.js" { }
        }
    }
}
