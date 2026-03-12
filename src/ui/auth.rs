//! Auth modal UI for the Quest Log Editor

use maud::{Markup, html};

/// Generate the auth modal HTML with the login form
pub fn auth_modal(error_message: Option<&str>, is_rate_limited: bool) -> Markup {
    html! {
        div id="auth-modal" class="auth-modal" data-signals="{_showError: false}" {
            div class="auth-backdrop" {
                div class="auth-container" {
                    div class="auth-header" {
                        h2 { "🗝️ Guild Entrance" }
                        p { "Enter your credentials to access the editor" }
                    }

                    @if is_rate_limited {
                        div class="auth-error" {
                            p { "⚠️ Too many login attempts. Please wait a minute before trying again." }
                        }
                    } @else if let Some(error) = error_message {
                        div class="auth-error" {
                            p { (error) }
                        }
                    }

                    form id="login-form" class="auth-form" data-signals="{_password: ''}" {
                        div class="form-group" {
                            label for="password" { "Master Key" }
                            input
                                type="password"
                                id="password"
                                name="password"
                                placeholder="Enter your master key..."
                                required
                                autocomplete="current-password"
                                data-bind="_password";
                        }

                        button
                            type="submit"
                            class="auth-submit"
                            data-on:click__prevent="@post('/editor/login', {contentType: 'form'})"
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
