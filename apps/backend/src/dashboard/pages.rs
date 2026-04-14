use axum::{
    Form,
    extract::State,
    http::header,
    response::{Html, IntoResponse, Redirect, Response},
};
use cookie::{Cookie, SameSite};
use serde::Deserialize;

use crate::repository::{FilterParams, ReadRepository};
use crate::state::AppState;

/// Check the access_token cookie and return the user email, or None.
pub async fn get_current_user(state: &AppState, headers: &axum::http::HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    let token = cookie_header.split(';').find_map(|c| {
        c.trim().strip_prefix("access_token=")
    })?;

    let token_data = state.jwt_service.validate_access_token(token).ok()?;
    Some(token_data.claims.email)
}

/// Main dashboard page
pub async fn dashboard(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let user = get_current_user(&state, &headers).await;

    if user.is_none() {
        return Redirect::to("/dashboard/login").into_response();
    }

    let email = user.unwrap();

    Html(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>rust_queue dashboard</title>
    <script src="https://unpkg.com/htmx.org@2.0.4"></script>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace;
            background: #0a0a0a;
            color: #e0e0e0;
            min-height: 100vh;
        }}

        .header {{
            border-bottom: 1px solid #222;
            padding: 16px 32px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}

        .header h1 {{
            font-size: 18px;
            font-weight: 600;
            color: #fff;
            letter-spacing: -0.5px;
        }}

        .header .user {{
            font-size: 13px;
            color: #666;
        }}

        .container {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 24px 32px;
        }}

        .section {{
            margin-bottom: 32px;
        }}

        .section-title {{
            font-size: 13px;
            font-weight: 600;
            color: #666;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 12px;
        }}

        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(5, 1fr);
            gap: 12px;
        }}

        .stat-card {{
            background: #111;
            border: 1px solid #222;
            border-radius: 8px;
            padding: 16px;
        }}

        .stat-card .label {{
            font-size: 12px;
            color: #666;
            margin-bottom: 4px;
        }}

        .stat-card .value {{
            font-size: 28px;
            font-weight: 700;
            font-variant-numeric: tabular-nums;
        }}

        .stat-card.pending .value {{ color: #f59e0b; }}
        .stat-card.running .value {{ color: #3b82f6; }}
        .stat-card.completed .value {{ color: #22c55e; }}
        .stat-card.dead .value {{ color: #ef4444; }}
        .stat-card.cancelled .value {{ color: #6b7280; }}

        .metrics-grid {{
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 12px;
        }}

        .metric-card {{
            background: #111;
            border: 1px solid #222;
            border-radius: 8px;
            padding: 16px;
        }}

        .metric-card .label {{
            font-size: 12px;
            color: #666;
            margin-bottom: 4px;
        }}

        .metric-card .value {{
            font-size: 20px;
            font-weight: 600;
            color: #fff;
        }}

        .metric-card .detail {{
            font-size: 12px;
            color: #555;
            margin-top: 4px;
        }}

        table {{
            width: 100%;
            border-collapse: collapse;
            font-size: 13px;
        }}

        thead th {{
            text-align: left;
            padding: 8px 12px;
            font-size: 11px;
            font-weight: 600;
            color: #666;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            border-bottom: 1px solid #222;
        }}

        tbody td {{
            padding: 10px 12px;
            border-bottom: 1px solid #1a1a1a;
            font-variant-numeric: tabular-nums;
        }}

        tbody tr:hover {{
            background: #111;
        }}

        .status {{
            display: inline-block;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 11px;
            font-weight: 600;
        }}

        .status.pending {{ background: #f59e0b20; color: #f59e0b; }}
        .status.running {{ background: #3b82f620; color: #3b82f6; }}
        .status.completed {{ background: #22c55e20; color: #22c55e; }}
        .status.dead {{ background: #ef444420; color: #ef4444; }}
        .status.cancelled {{ background: #6b728020; color: #6b7280; }}

        .mono {{ font-family: monospace; font-size: 12px; color: #888; }}

        .submit-form {{
            background: #111;
            border: 1px solid #222;
            border-radius: 8px;
            padding: 16px;
            display: flex;
            gap: 12px;
            align-items: end;
            flex-wrap: wrap;
        }}

        .form-group {{
            display: flex;
            flex-direction: column;
            gap: 4px;
        }}

        .form-group label {{
            font-size: 11px;
            color: #666;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}

        .form-group select,
        .form-group input {{
            background: #0a0a0a;
            border: 1px solid #333;
            border-radius: 4px;
            padding: 8px 12px;
            color: #e0e0e0;
            font-size: 13px;
            font-family: inherit;
        }}

        button {{
            background: #fff;
            color: #000;
            border: none;
            border-radius: 6px;
            padding: 8px 20px;
            font-size: 13px;
            font-weight: 600;
            cursor: pointer;
            font-family: inherit;
        }}

        button:hover {{ background: #ddd; }}

        .flash {{
            background: #22c55e15;
            border: 1px solid #22c55e40;
            color: #22c55e;
            padding: 8px 16px;
            border-radius: 6px;
            font-size: 13px;
            margin-bottom: 12px;
        }}

        .htmx-indicator {{
            opacity: 0;
            transition: opacity 200ms ease-in;
        }}

        .htmx-request .htmx-indicator {{
            opacity: 1;
        }}

        @media (max-width: 768px) {{
            .stats-grid {{ grid-template-columns: repeat(2, 1fr); }}
            .metrics-grid {{ grid-template-columns: 1fr; }}
            .container {{ padding: 16px; }}
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>rust_queue</h1>
        <span class="user">{email}</span>
    </div>

    <div class="container">
        <div class="section">
            <div class="section-title">Queue Status</div>
            <div hx-get="/dashboard/partials/stats"
                 hx-trigger="load, every 3s"
                 hx-swap="innerHTML">
            </div>
        </div>

        <div class="section">
            <div class="section-title">Metrics</div>
            <div hx-get="/dashboard/partials/metrics"
                 hx-trigger="load, every 5s"
                 hx-swap="innerHTML">
            </div>
        </div>

        <div class="section">
            <div class="section-title">Submit Job</div>
            <div hx-get="/dashboard/partials/submit"
                 hx-trigger="load"
                 hx-swap="innerHTML">
            </div>
        </div>

        <div class="section">
            <div class="section-title">Recent Jobs</div>
            <div hx-get="/dashboard/partials/jobs"
                 hx-trigger="load, every 3s"
                 hx-swap="innerHTML">
            </div>
        </div>
    </div>
</body>
</html>"#)).into_response()
}

/// Login page
pub async fn login_page() -> Html<String> {
    Html(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>rust_queue — login</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace;
            background: #0a0a0a;
            color: #e0e0e0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .login-box {
            background: #111;
            border: 1px solid #222;
            border-radius: 8px;
            padding: 32px;
            width: 100%;
            max-width: 360px;
        }
        h1 {
            font-size: 18px;
            font-weight: 600;
            margin-bottom: 24px;
            color: #fff;
        }
        .form-group {
            margin-bottom: 16px;
        }
        label {
            display: block;
            font-size: 11px;
            color: #666;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 6px;
        }
        input {
            width: 100%;
            background: #0a0a0a;
            border: 1px solid #333;
            border-radius: 4px;
            padding: 10px 12px;
            color: #e0e0e0;
            font-size: 14px;
            font-family: inherit;
        }
        input:focus { outline: none; border-color: #555; }
        button {
            width: 100%;
            background: #fff;
            color: #000;
            border: none;
            border-radius: 6px;
            padding: 10px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            margin-top: 8px;
            font-family: inherit;
        }
        button:hover { background: #ddd; }
        .error {
            background: #ef444420;
            border: 1px solid #ef444440;
            color: #ef4444;
            padding: 8px 12px;
            border-radius: 4px;
            font-size: 13px;
            margin-bottom: 16px;
        }
    </style>
</head>
<body>
    <div class="login-box">
        <h1>rust_queue</h1>
        <form method="POST" action="/dashboard/login">
            <div class="form-group">
                <label>Email</label>
                <input type="email" name="email" required autofocus>
            </div>
            <div class="form-group">
                <label>Password</label>
                <input type="password" name="password" required>
            </div>
            <button type="submit">Sign in</button>
        </form>
    </div>
</body>
</html>"#.to_string())
}

/// Handle login form submission
#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

pub async fn login_submit(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Response {
    // Find user by email
    let user = match state
        .users
        .find_one(&FilterParams::new().add_string("email", &form.email.to_lowercase()))
        .await
    {
        Ok(Some(user)) => user,
        _ => return Redirect::to("/dashboard/login").into_response(),
    };

    // Verify password
    let is_valid = state
        .password_service
        .verify(&form.password, &user.password_hash)
        .unwrap_or(false);

    if !is_valid {
        return Redirect::to("/dashboard/login").into_response();
    }

    // Generate tokens and set cookies
    let (access_token, refresh_token) = match state
        .jwt_service
        .generate_token_pair(user.id, &user.email)
    {
        Ok(tokens) => tokens,
        Err(_) => return Redirect::to("/dashboard/login").into_response(),
    };

    let access_cookie = Cookie::build(("access_token", access_token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::minutes(state.jwt_service.access_expiry_mins()))
        .build();

    let refresh_cookie = Cookie::build(("refresh_token", refresh_token))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/api/auth")
        .max_age(cookie::time::Duration::days(state.jwt_service.refresh_expiry_days()))
        .build();

    // Build the redirect response manually so we can attach multiple
    // Set-Cookie headers reliably. Using AppendHeaders with Redirect
    // ensures both cookies are set.
    let mut response = Redirect::to("/dashboard").into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        access_cookie.to_string().parse().unwrap(),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        refresh_cookie.to_string().parse().unwrap(),
    );
    response
}
