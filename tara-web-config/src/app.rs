use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::dark_mode::{DarkModeToggle, DarkModeToggleProps};

// Helper to register all our server functions, if we're in SSR mode
#[cfg(feature = "ssr")]
pub fn register_server_functions() {
    use crate::dark_mode::ToggleDarkMode;
    _ = ToggleDarkMode::register();
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context(cx);

    view! {
        cx,

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/leptos_start.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="" view=|cx| view! { cx, <HomePage/> }/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage(cx: Scope) -> impl IntoView {
    view! {cx,
        <h1> "Tara" </h1>
        <DarkModeToggle />
    }
}
