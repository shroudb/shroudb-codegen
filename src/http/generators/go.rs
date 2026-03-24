//! Go HTTP client generator.
//!
//! Produces a Go module with:
//! - `client.go`   — public `Client` struct, constructor, endpoint methods, `do` helper
//! - `errors.go`   — error types and `Is{Code}` helpers
//! - `types.go`    — response and options structs
//! - `go.mod`      — module definition
//! - `README.md`   — quick usage docs

use super::super::spec::{ApiSpec, EndpointDef};
use heck::ToUpperCamelCase;
use crate::generator::{GeneratedFile, Naming};
use std::fmt::Write;

use super::HttpGenerator;

pub struct GoGenerator;

impl HttpGenerator for GoGenerator {
    fn language(&self) -> &'static str {
        "Go"
    }

    fn generate(&self, spec: &ApiSpec, n: &Naming) -> Vec<GeneratedFile> {
        vec![
            gen_client(spec, n),
            gen_errors(spec, n),
            gen_types(spec, n),
            gen_go_mod(n),
            gen_readme(spec, n),
        ]
    }
}

// ─── Field type mapping ──────────────────────────────────────────────────────

fn go_type(field_type: &str) -> &'static str {
    match field_type {
        "string" => "string",
        "integer" => "int64",
        "json" => "map[string]any",
        "json_array" => "[]any",
        _ => "any",
    }
}

fn go_type_optional(field_type: &str) -> &'static str {
    match field_type {
        "string" => "*string",
        "integer" => "*int64",
        "json" => "map[string]any",
        "json_array" => "[]any",
        _ => "any",
    }
}

fn go_json_tag(field_name: &str, optional: bool) -> String {
    if optional {
        format!("`json:\"{field_name},omitempty\"`")
    } else {
        format!("`json:\"{field_name}\"`")
    }
}

fn auth_const(auth: &str) -> &'static str {
    match auth {
        "access_token" => "authAccess",
        "refresh_token" => "authRefresh",
        _ => "authNone",
    }
}

/// Build the Go format path expression from a spec path like "/auth/{keyspace}/signup".
/// Returns (format_string, args) — e.g. ("/auth/%s/signup", vec!["c.Keyspace"]).
fn go_path_expr(ep: &EndpointDef) -> (String, Vec<&'static str>) {
    if ep.has_keyspace() {
        let fmt_path = ep.path.replace("{keyspace}", "%s");
        (fmt_path, vec!["c.Keyspace"])
    } else {
        (ep.path.clone(), vec![])
    }
}

// ─── client.go ───────────────────────────────────────────────────────────────

fn gen_client(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut out = format!(
        r#"// Package {snake} provides a Go HTTP client for the {pascal} {description}.
//
// Auto-generated from {raw} protocol spec. Do not edit.
package {snake}

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

// Auth mode constants used by the do helper.
const (
	authNone    = 0
	authAccess  = 1
	authRefresh = 2
)

// Client is an HTTP client for the {pascal} {description}.
type Client struct {{
	// BaseURL is the root URL of the {pascal} server (e.g. "http://localhost:{port}").
	BaseURL string

	// Keyspace is the default keyspace used for endpoints that require one.
	Keyspace string

	// AccessToken is the current JWT access token. Updated automatically
	// after Signup, Login, and Refresh calls.
	AccessToken string

	// RefreshToken is the current opaque refresh token. Updated automatically
	// after Signup, Login, and Refresh calls.
	RefreshToken string

	// HTTPClient is the underlying HTTP client. Defaults to http.DefaultClient.
	HTTPClient *http.Client
}}

// ClientOption configures a Client.
type ClientOption func(*Client)

// WithKeyspace sets the default keyspace.
func WithKeyspace(ks string) ClientOption {{
	return func(c *Client) {{ c.Keyspace = ks }}
}}

// WithAccessToken sets the initial access token.
func WithAccessToken(t string) ClientOption {{
	return func(c *Client) {{ c.AccessToken = t }}
}}

// WithRefreshToken sets the initial refresh token.
func WithRefreshToken(t string) ClientOption {{
	return func(c *Client) {{ c.RefreshToken = t }}
}}

// WithHTTPClient sets a custom *http.Client.
func WithHTTPClient(hc *http.Client) ClientOption {{
	return func(c *Client) {{ c.HTTPClient = hc }}
}}

// NewClient creates a new {pascal} client.
//
// baseURL should include the scheme and host, e.g. "http://localhost:{port}".
func NewClient(baseURL string, opts ...ClientOption) *Client {{
	c := &Client{{
		BaseURL:    strings.TrimRight(baseURL, "/"),
		HTTPClient: http.DefaultClient,
	}}
	for _, opt := range opts {{
		opt(c)
	}}
	return c
}}

// do performs an HTTP request and decodes the JSON response.
func (c *Client) do(ctx context.Context, method, path string, body any, result any, authMode int) error {{
	url := c.BaseURL + path

	var bodyReader io.Reader
	if body != nil {{
		b, err := json.Marshal(body)
		if err != nil {{
			return fmt.Errorf("{snake}: marshal request: %w", err)
		}}
		bodyReader = bytes.NewReader(b)
	}}

	req, err := http.NewRequestWithContext(ctx, method, url, bodyReader)
	if err != nil {{
		return fmt.Errorf("{snake}: create request: %w", err)
	}}

	if body != nil {{
		req.Header.Set("Content-Type", "application/json")
	}}
	req.Header.Set("Accept", "application/json")

	switch authMode {{
	case authAccess:
		if c.AccessToken != "" {{
			req.Header.Set("Authorization", "Bearer "+c.AccessToken)
		}}
	case authRefresh:
		if c.RefreshToken != "" {{
			req.Header.Set("Authorization", "Bearer "+c.RefreshToken)
		}}
	}}

	resp, err := c.HTTPClient.Do(req)
	if err != nil {{
		return fmt.Errorf("{snake}: request %s %s: %w", method, path, err)
	}}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {{
		return fmt.Errorf("{snake}: read response: %w", err)
	}}

	if resp.StatusCode >= 400 {{
		var apiErr {pascal}Error
		if json.Unmarshal(respBody, &apiErr) == nil && apiErr.Code != "" {{
			return &apiErr
		}}
		return &{pascal}Error{{
			Code:    http.StatusText(resp.StatusCode),
			Message: string(respBody),
		}}
	}}

	if result != nil && len(respBody) > 0 {{
		if err := json.Unmarshal(respBody, result); err != nil {{
			return fmt.Errorf("{snake}: decode response: %w", err)
		}}
	}}

	return nil
}}
"#,
        pascal = n.pascal,
        snake = n.snake,
        raw = n.raw,
        port = n.default_port,
        description = n.description,
    );

    // Generate methods for each endpoint
    for (ep_name, ep) in &spec.endpoints {
        writeln!(out).unwrap();
        gen_go_method(&mut out, ep_name, ep, n);
    }

    GeneratedFile {
        path: "client.go".into(),
        content: out,
    }
}

fn gen_go_method(out: &mut String, ep_name: &str, ep: &EndpointDef, _n: &Naming) {
    let method_name = ep_name.to_upper_camel_case();
    let response_type = format!("{method_name}Response");
    let has_required_body = !ep.required_body().is_empty();
    let has_optional_body = !ep.optional_body().is_empty();
    let has_response = !ep.response.is_empty();

    // Build method signature parts
    let mut params = vec!["ctx context.Context".to_string()];

    // Required body fields become positional arguments
    for (name, _field) in ep.required_body() {
        let go_name = to_go_param_name(name);
        params.push(format!("{go_name} string"));
    }

    // Optional body fields become an options struct pointer
    if has_optional_body {
        let opts_type = format!("{method_name}Options");
        params.push(format!("opts *{opts_type}"));
    }

    let return_type = if has_response {
        format!("(*{response_type}, error)")
    } else {
        "error".into()
    };

    // Doc comment
    writeln!(out, "// {method_name} — {}", ep.description).unwrap();

    // Signature
    writeln!(
        out,
        "func (c *Client) {method_name}({}) {return_type} {{",
        params.join(", "),
    )
    .unwrap();

    // Build body map for POST requests
    if ep.method == "POST" && (has_required_body || has_optional_body) {
        writeln!(out, "\tbody := map[string]any{{").unwrap();
        for (name, _field) in ep.required_body() {
            let go_name = to_go_param_name(name);
            writeln!(out, "\t\t\"{name}\": {go_name},").unwrap();
        }
        writeln!(out, "\t}}").unwrap();

        // Merge optional fields from opts
        if has_optional_body {
            writeln!(out, "\tif opts != nil {{").unwrap();
            for (name, field) in ep.optional_body() {
                let field_name = name.to_upper_camel_case();
                let go_t = go_type(&field.field_type);
                match go_t {
                    "string" => {
                        writeln!(out, "\t\tif opts.{field_name} != \"\" {{").unwrap();
                        writeln!(out, "\t\t\tbody[\"{name}\"] = opts.{field_name}").unwrap();
                        writeln!(out, "\t\t}}").unwrap();
                    }
                    "int64" => {
                        writeln!(out, "\t\tif opts.{field_name} != 0 {{").unwrap();
                        writeln!(out, "\t\t\tbody[\"{name}\"] = opts.{field_name}").unwrap();
                        writeln!(out, "\t\t}}").unwrap();
                    }
                    _ => {
                        // map[string]any, []any, any
                        writeln!(out, "\t\tif opts.{field_name} != nil {{").unwrap();
                        writeln!(out, "\t\t\tbody[\"{name}\"] = opts.{field_name}").unwrap();
                        writeln!(out, "\t\t}}").unwrap();
                    }
                }
            }
            writeln!(out, "\t}}").unwrap();
        }
    }

    // Build path expression
    let (fmt_path, fmt_args) = go_path_expr(ep);
    let path_expr = if fmt_args.is_empty() {
        format!("\"{}\"", fmt_path)
    } else {
        format!(
            "fmt.Sprintf(\"{}\", {})",
            fmt_path,
            fmt_args.join(", ")
        )
    };

    let body_arg = if ep.method == "POST" && (has_required_body || has_optional_body) {
        "body"
    } else {
        "nil"
    };

    let auth = auth_const(&ep.auth);

    if has_response {
        writeln!(out, "\tresult := &{response_type}{{}}").unwrap();
        writeln!(
            out,
            "\terr := c.do(ctx, \"{}\", {path_expr}, {body_arg}, result, {auth})",
            ep.method
        )
        .unwrap();
        writeln!(out, "\tif err != nil {{").unwrap();
        writeln!(out, "\t\treturn nil, err").unwrap();
        writeln!(out, "\t}}").unwrap();

        // Auto-update tokens if the response contains them
        let resp_fields: Vec<&str> = ep.response.keys().map(|k| k.as_str()).collect();
        if resp_fields.contains(&"access_token") {
            writeln!(out, "\tc.AccessToken = result.AccessToken").unwrap();
        }
        if resp_fields.contains(&"refresh_token") {
            writeln!(out, "\tc.RefreshToken = result.RefreshToken").unwrap();
        }

        writeln!(out, "\treturn result, nil").unwrap();
    } else {
        writeln!(
            out,
            "\treturn c.do(ctx, \"{}\", {path_expr}, {body_arg}, nil, {auth})",
            ep.method
        )
        .unwrap();
    }

    writeln!(out, "}}").unwrap();
}

fn to_go_param_name(snake_name: &str) -> String {
    // Go convention: camelCase for parameters
    let parts: Vec<&str> = snake_name.split('_').collect();
    let mut result = parts[0].to_string();
    for part in &parts[1..] {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            result.extend(chars);
        }
    }
    // Avoid Go keywords
    match result.as_str() {
        "type" => "typ".into(),
        "func" => "fn".into(),
        _ => result,
    }
}

// ─── errors.go ───────────────────────────────────────────────────────────────

fn gen_errors(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut out = format!(
        r#"// {pascal} error types.
//
// Auto-generated from {raw} protocol spec. Do not edit.

package {snake}

import (
	"fmt"
)

// {pascal}Error represents an error returned by the {pascal} server.
type {pascal}Error struct {{
	// Machine-readable error code (e.g. "UNAUTHORIZED", "CONFLICT").
	Code string `json:"code"`

	// Human-readable error message.
	Message string `json:"message"`
}}

func (e *{pascal}Error) Error() string {{
	return fmt.Sprintf("[%s] %s", e.Code, e.Message)
}}

// Error code constants.
const (
"#,
        pascal = n.pascal,
        snake = n.snake,
        raw = n.raw,
    );

    for (code, def) in &spec.error_codes {
        let const_name = format!("Err{}", code.to_upper_camel_case());
        writeln!(out, "\t// {const_name} — {}", def.description).unwrap();
        writeln!(out, "\t{const_name} = \"{code}\"").unwrap();
    }

    out.push_str(")\n\n");

    // IsCode helper
    writeln!(
        out,
        r#"// IsCode reports whether the error has the given error code.
func IsCode(err error, code string) bool {{
	if ae, ok := err.(*{pascal}Error); ok {{
		return ae.Code == code
	}}
	return false
}}"#,
        pascal = n.pascal,
    )
    .unwrap();

    writeln!(out).unwrap();

    // Convenience Is{Code} helpers
    for (code, def) in &spec.error_codes {
        let helper_name = format!("Is{}", code.to_upper_camel_case());
        let const_name = format!("Err{}", code.to_upper_camel_case());
        writeln!(out).unwrap();
        writeln!(
            out,
            "// {helper_name} reports whether the error is {}: {}",
            code, def.description
        )
        .unwrap();
        writeln!(
            out,
            "func {helper_name}(err error) bool {{ return IsCode(err, {const_name}) }}"
        )
        .unwrap();
    }

    GeneratedFile {
        path: "errors.go".into(),
        content: out,
    }
}

// ─── types.go ────────────────────────────────────────────────────────────────

fn gen_types(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut out = format!(
        r#"// {pascal} request/response types.
//
// Auto-generated from {raw} protocol spec. Do not edit.

package {snake}

"#,
        pascal = n.pascal,
        snake = n.snake,
        raw = n.raw,
    );

    for (ep_name, ep) in &spec.endpoints {
        let pascal_name = ep_name.to_upper_camel_case();

        // Options struct for optional body fields
        let optional_body = ep.optional_body();
        if !optional_body.is_empty() {
            let opts_name = format!("{pascal_name}Options");
            writeln!(
                out,
                "// {opts_name} are optional parameters for {pascal_name}."
            )
            .unwrap();
            writeln!(out, "type {opts_name} struct {{").unwrap();
            for (name, field) in &optional_body {
                let field_name = name.to_upper_camel_case();
                let field_type = go_type(&field.field_type);
                let tag = go_json_tag(name, true);
                writeln!(
                    out,
                    "\t// {}", field.description
                )
                .unwrap();
                writeln!(out, "\t{field_name} {field_type} {tag}").unwrap();
            }
            writeln!(out, "}}\n").unwrap();
        }

        // Response struct
        if !ep.response.is_empty() {
            let response_name = format!("{pascal_name}Response");
            writeln!(
                out,
                "// {response_name} is the response from the {pascal_name} endpoint."
            )
            .unwrap();
            writeln!(out, "type {response_name} struct {{").unwrap();
            for (name, field) in &ep.response {
                let field_name = name.to_upper_camel_case();
                let field_type = if field.optional {
                    go_type_optional(&field.field_type)
                } else {
                    go_type(&field.field_type)
                };
                let tag = go_json_tag(name, field.optional);
                writeln!(out, "\t// {}", field.description).unwrap();
                writeln!(out, "\t{field_name} {field_type} {tag}").unwrap();
            }
            writeln!(out, "}}\n").unwrap();
        }
    }

    GeneratedFile {
        path: "types.go".into(),
        content: out,
    }
}

// ─── go.mod ──────────────────────────────────────────────────────────────────

fn gen_go_mod(n: &Naming) -> GeneratedFile {
    GeneratedFile {
        path: "go.mod".into(),
        content: format!("module {}\n\ngo 1.22\n", n.go_module),
    }
}

// ─── README.md ───────────────────────────────────────────────────────────────

fn gen_readme(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut methods = String::new();
    for (ep_name, ep) in &spec.endpoints {
        let method = ep_name.to_upper_camel_case();
        writeln!(methods, "- `client.{method}(...)` — {}", ep.description).unwrap();
    }

    GeneratedFile {
        path: "README.md".into(),
        content: format!(
            r#"# {pascal} Go Client

Go HTTP client for the [{pascal}](https://github.com/shroudb/{kebab}) {description}.

## Install

```bash
go get {go_module}
```

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"

    {snake} "{go_module}"
)

func main() {{
    client := {snake}.NewClient(
        "http://localhost:{port}",
        {snake}.WithKeyspace("my-keyspace"),
    )

    ctx := context.Background()

    // Sign up a user
    result, err := client.Signup(ctx, "alice", "s3cret", nil)
    if err != nil {{
        log.Fatal(err)
    }}
    fmt.Println("Access token:", result.AccessToken)

    // Check the session
    session, err := client.Session(ctx)
    if err != nil {{
        if {snake}.IsUnauthorized(err) {{
            log.Fatal("token expired — call Refresh()")
        }}
        log.Fatal(err)
    }}
    fmt.Println("User:", session.Claims)
}}
```

## Client Options

```go
client := {snake}.NewClient("http://localhost:{port}",
    {snake}.WithKeyspace("my-app"),
    {snake}.WithAccessToken("existing-jwt"),
    {snake}.WithRefreshToken("existing-rt"),
    {snake}.WithHTTPClient(&http.Client{{Timeout: 5 * time.Second}}),
)
```

## Endpoints

{methods}
## Error Handling

All API errors are returned as `*{snake}.{pascal}Error` with `Code` and `Message` fields.

```go
_, err := client.Login(ctx, "alice", "wrong-password")
if {snake}.IsUnauthorized(err) {{
    fmt.Println("bad credentials")
}}
```

## Auto-generated

This client was generated by `shroudb-auth-codegen` from `protocol.toml`.
"#,
            pascal = n.pascal,
            kebab = n.kebab,
            snake = n.snake,
            go_module = n.go_module,
            port = n.default_port,
            description = n.description,
            methods = methods,
        ),
    }
}
