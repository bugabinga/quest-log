//! Auth modal UI for the Quest Log Editor

use maud::{Markup, html};

/// Generate the auth modal HTML with the login form
#[must_use]
pub fn auth_modal() -> Markup {
    html! {
        div id="auth-modal" class="auth-modal" data-init="loginError = null; isRateLimited = false; _password = ''" {
            div class="auth-backdrop" {
                div class="auth-container" {
                    div class="auth-header" {
                        h2 { "🗝️ Guild Entrance" }
                        p { "Enter your credentials to access the editor" }
                    }

                    div class="auth-error" data-show="!!loginError" {
                        p data-text="loginError" {}
                    }

                    form id="login-form" class="auth-form" method="post" data-on:submit__prevent="(_password ?? '') && (_password ?? '').trim() !== '' ? @post('/editor/login') : (loginError = 'Please enter a password')" {
                        div class="form-group" {
                            label for="password" { "Master Key" }
                            input
                                type="password"
                                id="password"
                                name="password"
                                placeholder="Enter your master key..."
                                data-bind="_password"
                                autocomplete="current-password";
                        }

                        button
                            type="submit"
                            class="auth-submit"
                            data-indicator="#login-loading" {
                            span { "Enter the Guild" }
                            span id="login-loading" style="display: none" { "Authenticating..." }
                        }
                    }

                    div class="auth-footer" {
                        p { "Restricted access - for guild masters only" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_method_is_post() {
        // Regression test: form should have method="post" for security
        // GET requests expose passwords in URLs
        let markup = auth_modal();
        let html = markup.into_string();

        // The form element should specify method="post"
        assert!(
            html.contains(r#"method="post""#),
            "Expected form to have method=\"post\", got: {html}"
        );
    }

    #[test]
    fn test_submit_prevent_on_form_not_button() {
        // Regression test: data-on:submit__prevent should be on the form element,
        // not the submit button. Placing it on the button doesn't prevent form submission.
        let markup = auth_modal();
        let html = markup.into_string();

        // The form element should have data-on:submit__prevent
        assert!(
            html.contains(r"<form") && html.contains(r"data-on:submit__prevent="),
            "Expected data-on:submit__prevent on <form> element, got: {html}"
        );

        // The button should NOT have data-on:submit__prevent
        let button_section = html.split("<button").nth(1).unwrap_or("");
        assert!(
            !button_section.contains("data-on:submit__prevent"),
            "data-on:submit__prevent should not be on <button>, got: {html}"
        );
    }
}
