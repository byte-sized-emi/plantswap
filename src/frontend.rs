use axum_htmx::HxRequest;
use maud::{html, Markup};


pub async fn render_homepage(HxRequest(is_htmx): HxRequest) -> Markup {
    let content = html! {
        p {
            "A place for you to buy, sell, trade, and gift plants to other people."
        }
    };

    render_page(PageSelection::Home, content, is_htmx)
}

pub async fn render_about(HxRequest(is_htmx): HxRequest) -> Markup {
    let content = html! {
        p {
            "A place for you to buy, sell, trade, and gift plants to other people."
            "Made by Emilia Jaser."
            "Proudly free and open source."
        }
    };

    render_page(PageSelection::About, content, is_htmx)
}

fn render_page(page_selection: PageSelection, content: Markup, is_htmx: bool) -> Markup {
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
                        header { (render_header(page_selection))}
                        article #page { (content) }
                        footer { "Footer" }
                    }
                }
            }
        }
    }
}

fn render_header(page_selection: PageSelection) -> Markup {
    html! {
        div .header-site-name {
            "Plant swap"
        }
        (render_page_selector(page_selection))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PageSelection {
    Home,
    About,
}

impl PageSelection {
    fn render(&self) -> &str {
        use PageSelection::*;
        match self {
            Home => "Home",
            About => "About",
        }
    }

    fn href(&self) -> &str {
        use PageSelection::*;
        match self {
            Home => "/home",
            About => "/about",
        }
    }
}

const PAGE_SELECTIONS: &[PageSelection] = &[PageSelection::Home, PageSelection::About];

fn render_page_selector(current_selection: PageSelection) -> Markup {
    html! {
        nav #nav-selector hx-swap-oob="true" hx-replace-url="true" hx-target="#page" hx-swap="outerHTML" {
            @for selection in PAGE_SELECTIONS {
                a href=(selection.href()) hx-get=(selection.href()) .page-selector .current-page-selector[selection == &current_selection] {
                    (selection.render())
                }
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
