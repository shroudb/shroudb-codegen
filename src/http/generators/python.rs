//! Python HTTP client generator.
//!
//! Produces a self-contained Python package with:
//! - `client.py`      — async HTTP client using `httpx`
//! - `errors.py`      — exception hierarchy
//! - `types.py`       — dataclasses for endpoint responses
//! - `__init__.py`    — re-exports public API
//! - `pyproject.toml` — package metadata with `httpx` dependency
//! - `README.md`      — quick usage docs

use super::super::spec::{ApiSpec, EndpointDef};
use crate::generator::{GeneratedFile, Naming};
use heck::{ToPascalCase, ToSnakeCase};
use std::fmt::Write;

use super::HttpGenerator;

pub struct PythonGenerator;

impl HttpGenerator for PythonGenerator {
    fn language(&self) -> &'static str {
        "Python"
    }

    fn generate(&self, spec: &ApiSpec, n: &Naming) -> Vec<GeneratedFile> {
        vec![
            gen_client(spec, n),
            gen_errors(spec, n),
            gen_types(spec, n),
            gen_init(spec, n),
            gen_pyproject(spec, n),
            gen_readme(spec, n),
        ]
    }
}

/// Map spec field types to Python type annotations.
fn python_type(field_type: &str) -> &'static str {
    match field_type {
        "string" => "str",
        "integer" => "int",
        "json" => "dict[str, Any]",
        "json_array" => "list[Any]",
        _ => "Any",
    }
}

/// Default value for a Python type in a dataclass field.
fn py_default(field_type: &str) -> &'static str {
    match field_type {
        "string" => "\"\"",
        "integer" => "0",
        "json" => "field(default_factory=dict)",
        "json_array" => "field(default_factory=list)",
        _ => "None",
    }
}

/// Convert an error code like "TOO_MANY_REQUESTS" to a class name like "TooManyRequestsError".
fn code_to_class(code: &str) -> String {
    let mut name = String::new();
    for part in code.split('_') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            name.push(first.to_ascii_uppercase());
            for c in chars {
                name.push(c.to_ascii_lowercase());
            }
        }
    }
    name.push_str("Error");
    name
}

fn to_pascal(s: &str) -> String {
    s.to_pascal_case()
}

/// Whether an endpoint returns tokens that should be stored on the client.
fn stores_tokens(ep: &EndpointDef) -> bool {
    ep.response.contains_key("access_token") && ep.response.contains_key("refresh_token")
}

// ─── client.py ───────────────────────────────────────────────────────────────

fn gen_client(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut out = format!(
        r#""""
{pascal} HTTP client.

Auto-generated from {raw} API spec. Do not edit.
"""
from __future__ import annotations

from typing import Any, Optional

import httpx

from .errors import {pascal}Error
"#,
        pascal = n.pascal,
        raw = n.raw,
    );

    // Import response types
    let mut imports = Vec::new();
    for (ep_name, ep) in &spec.endpoints {
        if !ep.response.is_empty() {
            imports.push(format!("{}Response", to_pascal(ep_name)));
        }
    }
    if !imports.is_empty() {
        writeln!(out, "from .types import {}", imports.join(", ")).unwrap();
    }

    // Client class
    write!(
        out,
        r#"

class {pascal}Client:
    """Async HTTP client for the {pascal} {description}.

    Usage::

        async with {pascal}Client("http://localhost:{port}") as client:
            result = await client.signup("alice", "s3cret")
            print(result.access_token)
    """

    def __init__(
        self,
        base_url: str,
        *,
        keyspace: Optional[str] = None,
        timeout: float = 30.0,
    ) -> None:
        """Create a new {pascal} client.

        Args:
            base_url: Base URL of the {pascal} server (e.g. ``"http://localhost:{port}"``).
            keyspace: Default keyspace for all requests. Can be overridden per call.
            timeout: Request timeout in seconds (default: 30).
        """
        self._base_url = base_url.rstrip("/")
        self._keyspace = keyspace
        self._http = httpx.AsyncClient(timeout=timeout)
        self._access_token: Optional[str] = None
        self._refresh_token: Optional[str] = None

    @property
    def access_token(self) -> Optional[str]:
        """Current access token, if any."""
        return self._access_token

    @access_token.setter
    def access_token(self, value: Optional[str]) -> None:
        self._access_token = value

    @property
    def refresh_token(self) -> Optional[str]:
        """Current refresh token, if any."""
        return self._refresh_token

    @refresh_token.setter
    def refresh_token(self, value: Optional[str]) -> None:
        self._refresh_token = value

    async def close(self) -> None:
        """Close the underlying HTTP client."""
        await self._http.aclose()

    async def __aenter__(self) -> "{pascal}Client":
        return self

    async def __aexit__(self, *args: Any) -> None:
        await self.close()

    def _resolve_keyspace(self, keyspace: Optional[str]) -> str:
        """Return the keyspace to use, raising if none provided."""
        ks = keyspace or self._keyspace
        if ks is None:
            raise ValueError(
                "No keyspace provided. Pass keyspace= to the method or set it in the constructor."
            )
        return ks

    def _auth_headers(self) -> dict[str, str]:
        """Headers for endpoints that require access_token auth."""
        if self._access_token is None:
            raise {pascal}Error("UNAUTHORIZED", "No access token set. Log in or sign up first.")
        return {{"Authorization": f"Bearer {{self._access_token}}"}}

    def _refresh_headers(self) -> dict[str, str]:
        """Headers for endpoints that require refresh_token auth."""
        if self._refresh_token is None:
            raise {pascal}Error("UNAUTHORIZED", "No refresh token set. Log in or sign up first.")
        return {{"Authorization": f"Bearer {{self._refresh_token}}"}}

    async def _request(
        self,
        method: str,
        path: str,
        *,
        json: Optional[dict[str, Any]] = None,
        headers: Optional[dict[str, str]] = None,
        expected_status: int = 200,
    ) -> dict[str, Any]:
        """Send an HTTP request and return the parsed JSON response.

        Raises:
            {pascal}Error: On non-success status codes with error body.
        """
        url = f"{{self._base_url}}{{path}}"
        hdrs: dict[str, str] = dict(headers or {{}})
        resp = await self._http.request(method, url, json=json, headers=hdrs)
        if resp.status_code >= 400:
            try:
                data = resp.json()
                raise {pascal}Error._from_response(resp.status_code, data)
            except ({pascal}Error, ):
                raise
            except (ValueError, KeyError):
                body_text = resp.text[:500]
                raise {pascal}Error("HTTP_ERROR", body_text)
        if resp.status_code != expected_status:
            body_text = resp.text[:500]
            raise {pascal}Error(
                "UNEXPECTED_STATUS",
                f"expected {{expected_status}}, got {{resp.status_code}}: {{body_text}}",
            )
        if resp.status_code == 204 or not resp.content:
            return {{}}
        return resp.json()
"#,
        pascal = n.pascal,
        description = n.description,
        port = n.default_port,
    )
    .unwrap();

    // Generate a method for each endpoint
    for (ep_name, ep) in &spec.endpoints {
        writeln!(out).unwrap();
        gen_python_method(&mut out, ep_name, ep);
    }

    GeneratedFile {
        path: format!("{}/client.py", n.snake),
        content: out,
    }
}

fn gen_python_method(out: &mut String, ep_name: &str, ep: &EndpointDef) {
    let method_name = ep_name.to_snake_case();
    let has_keyspace = ep.has_keyspace();
    let required_body = ep.required_body();
    let optional_body = ep.optional_body();
    let path_params = ep.path_params();

    // Non-keyspace path params become method parameters
    let extra_path_params: Vec<String> = path_params
        .iter()
        .filter(|p| *p != "keyspace")
        .cloned()
        .collect();

    // Build signature
    let mut sig_parts: Vec<String> = vec!["self".into()];

    // Path params (other than keyspace) are positional
    for param in &extra_path_params {
        sig_parts.push(format!("{param}: str"));
    }

    // Required body params are positional
    for (name, field) in &required_body {
        let py_type = python_type(&field.field_type);
        sig_parts.push(format!("{name}: {py_type}"));
    }

    // Keyword-only separator before optional params and keyspace
    let has_optional = !optional_body.is_empty() || has_keyspace;
    if has_optional {
        sig_parts.push("*".into());
    }

    // Optional body params
    for (name, field) in &optional_body {
        let py_type = python_type(&field.field_type);
        sig_parts.push(format!("{name}: Optional[{py_type}] = None"));
    }

    // Keyspace override
    if has_keyspace {
        sig_parts.push("keyspace: Optional[str] = None".into());
    }

    // Return type
    let return_type = if ep.response.is_empty() {
        "None".to_string()
    } else {
        format!("{}Response", to_pascal(ep_name))
    };

    writeln!(
        out,
        "    async def {method_name}({}) -> {return_type}:",
        sig_parts.join(", ")
    )
    .unwrap();
    writeln!(out, "        \"\"\"{}\"\"\"", ep.description).unwrap();

    // Build path
    if has_keyspace {
        writeln!(out, "        ks = self._resolve_keyspace(keyspace)").unwrap();
    }
    if !path_params.is_empty() {
        // Build an f-string that interpolates all path params
        let mut path_template = ep.path.clone();
        if has_keyspace {
            path_template = path_template.replace("{keyspace}", "{ks}");
        }
        writeln!(out, "        path = f\"{path_template}\"").unwrap();
    } else {
        writeln!(out, "        path = \"{}\"", ep.path).unwrap();
    }

    // Build body
    if ep.has_body() && (!required_body.is_empty() || !optional_body.is_empty()) {
        write!(out, "        body: dict[str, Any] = {{").unwrap();
        let mut first = true;
        for (name, _) in &required_body {
            if !first {
                write!(out, ", ").unwrap();
            }
            write!(out, "\"{name}\": {name}").unwrap();
            first = false;
        }
        writeln!(out, "}}").unwrap();

        for (name, _) in &optional_body {
            writeln!(out, "        if {name} is not None:").unwrap();
            writeln!(out, "            body[\"{name}\"] = {name}").unwrap();
        }
    }

    // Auth headers
    let headers_arg = match ep.auth.as_str() {
        "access_token" => "headers=self._auth_headers()",
        "refresh_token" => "headers=self._refresh_headers()",
        "none" => "",
        other => panic!("unknown auth type '{other}' on endpoint {ep_name}"),
    };

    // Make request
    let has_body_fields = ep.has_body() && (!required_body.is_empty() || !optional_body.is_empty());
    let expected_status = ep.success_status;
    if has_body_fields {
        if headers_arg.is_empty() {
            writeln!(
                out,
                "        data = await self._request(\"{}\", path, json=body, expected_status={expected_status})",
                ep.method
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "        data = await self._request(\"{}\", path, json=body, {headers_arg}, expected_status={expected_status})",
                ep.method
            )
            .unwrap();
        }
    } else if headers_arg.is_empty() {
        writeln!(
            out,
            "        data = await self._request(\"{}\", path, expected_status={expected_status})",
            ep.method
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "        data = await self._request(\"{}\", path, {headers_arg}, expected_status={expected_status})",
            ep.method
        )
        .unwrap();
    }

    // Store tokens if the response contains them
    if stores_tokens(ep) {
        writeln!(out, "        result = {return_type}._from_dict(data)").unwrap();
        writeln!(out, "        self._access_token = result.access_token").unwrap();
        writeln!(out, "        self._refresh_token = result.refresh_token").unwrap();
        writeln!(out, "        return result").unwrap();
    } else if !ep.response.is_empty() {
        writeln!(out, "        return {return_type}._from_dict(data)").unwrap();
    }

    writeln!(out).unwrap();
}

// ─── errors.py ───────────────────────────────────────────────────────────────

fn gen_errors(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut out = format!(
        r#""""
{pascal} error types.

Auto-generated from {raw} API spec. Do not edit.
"""
from __future__ import annotations

from typing import Any


class {pascal}Error(Exception):
    """Base exception for all {pascal} operations."""

    def __init__(self, code: str, message: str) -> None:
        self.code = code
        self.message = message
        super().__init__(f"[{{code}}] {{message}}")

    @classmethod
    def _from_response(cls, status_code: int, data: dict[str, Any]) -> "{pascal}Error":
        """Construct the appropriate error subclass from an HTTP error response.

        Expected response body: ``{{"error": "CODE", "message": "..."}}``
        """
        code = data.get("error", "UNKNOWN")
        message = data.get("message", data.get("error", f"HTTP {{status_code}}"))
        subclass = _ERROR_MAP.get(code, {pascal}Error)
        return subclass(code, message)

"#,
        pascal = n.pascal,
        raw = n.raw,
    );

    for (code, def) in &spec.error_codes {
        let class_name = code_to_class(code);
        writeln!(out).unwrap();
        writeln!(
            out,
            "class {class_name}({pascal}Error):\n    \"\"\"{}\"\"\"\n",
            def.description,
            pascal = n.pascal,
        )
        .unwrap();
    }

    out.push_str(&format!(
        "\n_ERROR_MAP: dict[str, type[{pascal}Error]] = {{\n",
        pascal = n.pascal,
    ));
    for code in spec.error_codes.keys() {
        let class_name = code_to_class(code);
        writeln!(out, "    \"{code}\": {class_name},").unwrap();
    }
    out.push_str("}\n");

    GeneratedFile {
        path: format!("{}/errors.py", n.snake),
        content: out,
    }
}

// ─── types.py ────────────────────────────────────────────────────────────────

fn gen_types(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut out = format!(
        r#""""
{pascal} response types.

Auto-generated from {raw} API spec. Do not edit.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional

"#,
        pascal = n.pascal,
        raw = n.raw,
    );

    for (ep_name, ep) in &spec.endpoints {
        if ep.response.is_empty() {
            continue;
        }
        let class_name = format!("{}Response", to_pascal(ep_name));
        writeln!(out, "@dataclass").unwrap();
        writeln!(out, "class {class_name}:").unwrap();
        writeln!(out, "    \"\"\"Response from {}.\"\"\"", ep.description).unwrap();
        writeln!(out).unwrap();

        // Required fields first, then optional
        for (fname, fdef) in ep.required_response() {
            let py_type = python_type(&fdef.field_type);
            writeln!(
                out,
                "    {fname}: {py_type} = {}",
                py_default(&fdef.field_type)
            )
            .unwrap();
        }
        for (fname, fdef) in ep.optional_response() {
            let py_type = python_type(&fdef.field_type);
            writeln!(out, "    {fname}: Optional[{py_type}] = None").unwrap();
        }

        writeln!(out).unwrap();
        writeln!(out, "    @classmethod").unwrap();
        writeln!(
            out,
            "    def _from_dict(cls, data: dict[str, Any]) -> \"{class_name}\":"
        )
        .unwrap();
        writeln!(out, "        return cls(").unwrap();
        for (fname, fdef) in ep.required_response() {
            let _ = fdef;
            writeln!(out, "            {fname}=data[\"{fname}\"],").unwrap();
        }
        for (fname, fdef) in ep.optional_response() {
            let _ = fdef;
            writeln!(out, "            {fname}=data.get(\"{fname}\"),").unwrap();
        }
        writeln!(out, "        )").unwrap();
        writeln!(out).unwrap();
        writeln!(out).unwrap();
    }

    GeneratedFile {
        path: format!("{}/types.py", n.snake),
        content: out,
    }
}

// ─── __init__.py ─────────────────────────────────────────────────────────────

fn gen_init(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut out = format!(
        r#""""
{pascal} — Python client for the {pascal} {description}.

Auto-generated from {raw} API spec. Do not edit.

Usage::

    from {snake} import {pascal}Client

    async with {pascal}Client("http://localhost:{port}") as client:
        result = await client.signup("alice", "s3cret")
        print(result.access_token)
"""
from .client import {pascal}Client
from .errors import {pascal}Error
"#,
        pascal = n.pascal,
        raw = n.raw,
        snake = n.snake,
        port = n.default_port,
        description = n.description,
    );

    // Re-export response types
    let mut type_names = Vec::new();
    for (ep_name, ep) in &spec.endpoints {
        if !ep.response.is_empty() {
            type_names.push(format!("{}Response", to_pascal(ep_name)));
        }
    }
    if !type_names.is_empty() {
        writeln!(out, "from .types import {}", type_names.join(", ")).unwrap();
    }

    // Re-export specific error classes
    let mut error_names = Vec::new();
    for code in spec.error_codes.keys() {
        error_names.push(code_to_class(code));
    }
    if !error_names.is_empty() {
        writeln!(out, "from .errors import {}", error_names.join(", ")).unwrap();
    }

    writeln!(out).unwrap();
    writeln!(out, "__version__ = \"{}\"", spec.api.version).unwrap();

    // __all__
    writeln!(out).unwrap();
    writeln!(out, "__all__ = [").unwrap();
    writeln!(out, "    \"{}Client\",", n.pascal).unwrap();
    writeln!(out, "    \"{}Error\",", n.pascal).unwrap();
    for name in &type_names {
        writeln!(out, "    \"{name}\",").unwrap();
    }
    for name in &error_names {
        writeln!(out, "    \"{name}\",").unwrap();
    }
    writeln!(out, "]").unwrap();

    GeneratedFile {
        path: format!("{}/__init__.py", n.snake),
        content: out,
    }
}

// ─── pyproject.toml ──────────────────────────────────────────────────────────

fn gen_pyproject(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    GeneratedFile {
        path: "pyproject.toml".into(),
        content: format!(
            r#"[project]
name = "{kebab}"
version = "{version}"
description = "Python client for the {pascal} {description}"
requires-python = ">=3.10"
license = "MIT"
dependencies = [
    "httpx>=0.25",
]

[build-system]
requires = ["setuptools>=68"]
build-backend = "setuptools.build_meta"
"#,
            kebab = n.kebab,
            version = spec.api.version,
            pascal = n.pascal,
            description = n.description,
        ),
    }
}

// ─── README.md ───────────────────────────────────────────────────────────────

fn gen_readme(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut methods = String::new();
    for (ep_name, ep) in &spec.endpoints {
        let method = ep_name.to_snake_case();
        methods.push_str(&format!("- `client.{method}(...)` — {}\n", ep.description));
    }

    // Build a spec-driven quick-start example from the first POST endpoint with a body
    let mut example = String::new();
    for (ep_name, ep) in &spec.endpoints {
        if !ep.has_body() || ep.required_body().is_empty() {
            continue;
        }
        let method = ep_name.to_snake_case();
        let mut args: Vec<String> = Vec::new();
        for (name, _field) in ep.required_body() {
            let example_val = match name {
                "user_id" => "\"alice\"",
                "password" => "\"s3cret\"",
                _ => "\"...\"",
            };
            args.push(example_val.to_string());
        }
        writeln!(
            example,
            "        result = await client.{method}({})",
            args.join(", ")
        )
        .unwrap();
        if !ep.response.is_empty() {
            writeln!(example, "        print(result)").unwrap();
        }
        break;
    }

    GeneratedFile {
        path: "README.md".into(),
        content: format!(
            r#"# {pascal} Python Client

Python client for the [{pascal}](https://github.com/shroudb/{kebab}) {description}.

## Install

```bash
pip install {kebab}
```

## Quick Start

```python
import asyncio
from {snake} import {pascal}Client

async def main():
    async with {pascal}Client("http://localhost:{port}") as client:
{example}
asyncio.run(main())
```

## Constructor

```python
client = {pascal}Client(
    "http://localhost:{port}",
    keyspace="my-app",  # default keyspace for all requests
    timeout=30.0,       # request timeout in seconds
)
```

## Token Management

After calling an endpoint that returns tokens, the client automatically stores
the access and refresh tokens. Subsequent calls to authenticated endpoints use
these tokens automatically.

You can also manage tokens manually:

```python
client.access_token = "eyJ..."
client.refresh_token = "rt_..."
```

## Methods

{methods}

## Async Context Manager

```python
async with {pascal}Client("http://localhost:{port}") as client:
    ...
```

## Auto-generated

This client was generated by `shroudb-codegen` from the `{raw}` protocol spec.
"#,
            pascal = n.pascal,
            kebab = n.kebab,
            snake = n.snake,
            port = n.default_port,
            description = n.description,
            methods = methods,
            example = example,
            raw = n.raw,
        ),
    }
}
