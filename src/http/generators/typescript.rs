//! TypeScript HTTP client generator.
//!
//! Produces an npm package with:
//! - `src/client.ts`   — public client class using `fetch`
//! - `src/errors.ts`   — error class hierarchy
//! - `src/types.ts`    — TypeScript interfaces for responses
//! - `src/index.ts`    — re-exports public API
//! - `package.json`    — npm metadata
//! - `tsconfig.json`   — TypeScript config
//! - `README.md`       — quick usage docs

use super::super::spec::{ApiSpec, EndpointDef, FieldDef};
use crate::generator::{GeneratedFile, Naming};
use heck::ToLowerCamelCase;
use std::fmt::Write;

use super::HttpGenerator;

pub struct TypeScriptGenerator;

impl HttpGenerator for TypeScriptGenerator {
    fn language(&self) -> &'static str {
        "TypeScript"
    }

    fn generate(&self, spec: &ApiSpec, n: &Naming) -> Vec<GeneratedFile> {
        vec![
            gen_errors(spec, n),
            gen_types(spec, n),
            gen_client(spec, n),
            gen_index(spec, n),
            gen_package_json(spec, n),
            gen_tsconfig(n),
            gen_readme(spec, n),
        ]
    }
}

// ─── helpers ────────────────────────────────────────────────────────────────

/// Map a spec field type to its TypeScript representation.
fn ts_type(field: &FieldDef) -> &'static str {
    match field.field_type.as_str() {
        "string" => "string",
        "integer" => "number",
        "json" => "Record<string, unknown>",
        "json_array" => "unknown[]",
        _ => "unknown",
    }
}

/// Convert a snake_case field name to camelCase for TypeScript.
fn ts_field(name: &str) -> String {
    name.to_lower_camel_case()
}

/// Convert an endpoint name to a camelCase method name.
fn ts_method(name: &str) -> String {
    name.to_lower_camel_case()
}

/// Build a PascalCase response interface name from an endpoint name.
fn ts_response_iface(endpoint_name: &str) -> String {
    use heck::ToPascalCase;
    format!("{}Response", endpoint_name.to_pascal_case())
}

/// Build the TypeScript path expression for an endpoint, substituting
/// `{keyspace}` with `${this.keyspace}`.
fn ts_path_expr(ep: &EndpointDef) -> String {
    let replaced = ep.path.replace("{keyspace}", "${this.keyspace}");
    format!("`{replaced}`")
}

/// Header comment placed atop every generated file.
fn file_header(pascal: &str) -> String {
    format!("// {pascal} TypeScript client — auto-generated from API spec. Do not edit.\n")
}

// ─── src/errors.ts ──────────────────────────────────────────────────────────

fn gen_errors(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(4096);
    let _ = write!(s, "{}", file_header(&n.pascal));

    // Base error class
    let _ = write!(
        s,
        r#"
/**
 * Base error thrown by the {pascal} client.
 */
export class {pascal}Error extends Error {{
  /** Machine-readable error code. */
  public readonly code: string;
  /** Optional human-readable detail from the server. */
  public readonly detail: string | undefined;
  /** HTTP status code, if available. */
  public readonly status: number | undefined;

  constructor(code: string, message: string, detail?: string, status?: number) {{
    super(message);
    this.name = "{pascal}Error";
    this.code = code;
    this.detail = detail;
    this.status = status;
    Object.setPrototypeOf(this, new.target.prototype);
  }}

  /**
   * Construct the appropriate error subclass from an error response body.
   * @internal
   */
  static _fromResponse(body: Record<string, unknown>, status: number): {pascal}Error {{
    const code = (body.code as string) ?? "UNKNOWN";
    const message = (body.message as string) ?? (body.error as string) ?? "Unknown error";
    const detail = body.detail as string | undefined;
    switch (code) {{
"#,
        pascal = n.pascal,
    );

    // One case per error code
    for code in spec.error_codes.keys() {
        use heck::ToPascalCase;
        let class = format!("{}Error", code.to_pascal_case());
        let _ = writeln!(
            s,
            "      case \"{code}\": return new {class}(message, detail, status);",
        );
    }

    let _ = write!(
        s,
        r#"      default: return new {pascal}Error(code, message, detail, status);
    }}
  }}
}}
"#,
        pascal = n.pascal,
    );

    // Subclass per error code
    for (code, def) in &spec.error_codes {
        use heck::ToPascalCase;
        let class = format!("{}Error", code.to_pascal_case());
        let _ = write!(
            s,
            r#"
/** {description} */
export class {class} extends {pascal}Error {{
  constructor(message: string, detail?: string, status?: number) {{
    super("{code}", message, detail, status);
    this.name = "{class}";
  }}
}}
"#,
            pascal = n.pascal,
            description = def.description,
        );
    }

    GeneratedFile {
        path: format!("{}/src/errors.ts", n.npm_name),
        content: s,
    }
}

// ─── src/types.ts ───────────────────────────────────────────────────────────

fn gen_types(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(4096);
    let _ = write!(s, "{}", file_header(&n.pascal));
    let _ = writeln!(s);

    for (ep_name, ep) in &spec.endpoints {
        if ep.response.is_empty() {
            continue;
        }
        let iface = ts_response_iface(ep_name);
        let _ = writeln!(s, "/** Response from the `{ep_name}` endpoint. */");
        let _ = writeln!(s, "export interface {iface} {{");
        for (field_name, field) in &ep.response {
            let ty = ts_type(field);
            let opt = if field.optional { "?" } else { "" };
            let _ = writeln!(s, "  /** {desc} */", desc = field.description);
            let _ = writeln!(s, "  {field_name}{opt}: {ty};");
        }
        let _ = writeln!(s, "}}\n");
    }

    GeneratedFile {
        path: format!("{}/src/types.ts", n.npm_name),
        content: s,
    }
}

// ─── src/client.ts ──────────────────────────────────────────────────────────

fn gen_client(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(16384);
    let _ = write!(s, "{}", file_header(&n.pascal));

    // Imports
    let _ = writeln!(
        s,
        "import {{ {pascal}Error }} from \"./errors\";",
        pascal = n.pascal
    );

    // Collect response interface names that we actually emit
    let response_imports: Vec<String> = spec
        .endpoints
        .iter()
        .filter(|(_, ep)| !ep.response.is_empty())
        .map(|(name, _)| ts_response_iface(name))
        .collect();

    if !response_imports.is_empty() {
        let _ = write!(s, "import type {{ ");
        for (i, name) in response_imports.iter().enumerate() {
            if i > 0 {
                let _ = write!(s, ", ");
            }
            let _ = write!(s, "{name}");
        }
        let _ = writeln!(s, " }} from \"./types\";");
    }

    let _ = writeln!(s);

    // Options interface for methods with optional body fields
    gen_options_interfaces(spec, &mut s);

    // Client class
    let _ = write!(
        s,
        r#"/**
 * {description}
 *
 * Uses the Fetch API — works in browsers, Node 18+, Deno, and Bun.
 */
export class {pascal}Client {{
  private readonly baseUrl: string;
  private readonly keyspace: string;
  /** Request timeout in milliseconds. */
  private readonly timeout: number;
  /** Current access token, set automatically after auth calls. */
  public accessToken: string | undefined;
  /** Current refresh token, set automatically after auth calls. */
  public refreshToken: string | undefined;

  /**
   * Create a new {pascal} client.
   * @param baseUrl  Base URL of the {pascal} server (e.g. `http://localhost:{port}`).
   * @param keyspace Optional keyspace. Defaults to `"default"`.
   * @param timeout  Request timeout in milliseconds. Defaults to `30000`.
   */
  constructor(baseUrl: string, keyspace?: string, timeout?: number) {{
    // Strip trailing slash for consistent URL building
    this.baseUrl = baseUrl.replace(/\/+$/, "");
    this.keyspace = keyspace ?? "default";
    this.timeout = timeout ?? 30_000;
  }}
"#,
        pascal = n.pascal,
        description = n.description,
        port = n.default_port,
    );

    // Generate a method per endpoint
    for (ep_name, ep) in &spec.endpoints {
        gen_endpoint_method(spec, n, ep_name, ep, &mut s);
    }

    // Private request helper
    let _ = write!(
        s,
        r#"
  // ─── internal ───────────────────────────────────────────────────────

  private async request(
    method: string,
    path: string,
    opts?: {{ body?: Record<string, unknown>; auth?: string }},
  ): Promise<Record<string, unknown>> {{
    const url = `${{this.baseUrl}}${{path}}`;
    const headers: Record<string, string> = {{}};

    if (opts?.auth !== undefined && opts.auth !== "") {{
      headers["Authorization"] = `Bearer ${{opts.auth}}`;
    }}

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeout);

    const init: RequestInit = {{ method, headers, signal: controller.signal }};
    if (opts?.body !== undefined) {{
      headers["Content-Type"] = "application/json";
      init.body = JSON.stringify(opts.body);
    }}

    let res: Response;
    try {{
      res = await fetch(url, init);
    }} catch (err) {{
      clearTimeout(timer);
      if (err instanceof DOMException && err.name === "AbortError") {{
        throw new {pascal}Error(
          "TIMEOUT",
          `Request timed out after ${{this.timeout}}ms`,
          undefined,
          undefined,
        );
      }}
      throw err;
    }} finally {{
      clearTimeout(timer);
    }}

    const text = await res.text();
    const truncate = (s: string, max: number) =>
      s.length > max ? s.slice(0, max) + "..." : s;
    let json: Record<string, unknown>;
    try {{
      json = JSON.parse(text) as Record<string, unknown>;
    }} catch {{
      throw new {pascal}Error(
        "PARSE_ERROR",
        `Server returned non-JSON response (HTTP ${{res.status}})`,
        truncate(text, 500),
        res.status,
      );
    }}

    if (!res.ok) {{
      throw {pascal}Error._fromResponse(json, res.status);
    }}

    return json;
  }}
}}
"#,
        pascal = n.pascal,
    );

    GeneratedFile {
        path: format!("{}/src/client.ts", n.npm_name),
        content: s,
    }
}

/// Generate TypeScript option interfaces for endpoints that have optional body fields.
fn gen_options_interfaces(spec: &ApiSpec, s: &mut String) {
    for (ep_name, ep) in &spec.endpoints {
        let optional = ep.optional_body();
        if optional.is_empty() {
            continue;
        }
        use heck::ToPascalCase;
        let iface = format!("{}Options", ep_name.to_pascal_case());
        let _ = writeln!(
            s,
            "/** Optional parameters for `{method}`. */",
            method = ts_method(ep_name)
        );
        let _ = writeln!(s, "export interface {iface} {{");
        for (field_name, field) in &optional {
            let ty = ts_type(field);
            let _ = writeln!(s, "  /** {desc} */", desc = field.description);
            let _ = writeln!(s, "  {field_name}?: {ty};");
        }
        let _ = writeln!(s, "}}\n");
    }
}

/// Generate a single endpoint method on the client class.
fn gen_endpoint_method(
    _spec: &ApiSpec,
    _n: &Naming,
    ep_name: &str,
    ep: &EndpointDef,
    s: &mut String,
) {
    let method_name = ts_method(ep_name);
    let required = ep.required_body();
    let optional = ep.optional_body();
    let has_optional = !optional.is_empty();
    let has_response = !ep.response.is_empty();
    let return_type = if has_response {
        ts_response_iface(ep_name)
    } else {
        "void".to_string()
    };

    // Build parameter list
    let mut params = Vec::new();
    for (field_name, field) in &required {
        let ts_name = ts_field(field_name);
        let ty = ts_type(field);
        params.push(format!("{ts_name}: {ty}"));
    }
    if has_optional {
        use heck::ToPascalCase;
        let opts_iface = format!("{}Options", ep_name.to_pascal_case());
        params.push(format!("options?: {opts_iface}"));
    }
    let params_str = params.join(", ");

    // JSDoc
    let _ = writeln!(s);
    let _ = writeln!(s, "  /**");
    let _ = writeln!(s, "   * {}", ep.description);
    for (field_name, field) in &required {
        let ts_name = ts_field(field_name);
        let _ = writeln!(s, "   * @param {ts_name} {}", field.description);
    }
    if has_optional {
        let _ = writeln!(s, "   * @param options Optional parameters.");
    }
    let _ = writeln!(s, "   */");

    // Signature
    let _ = writeln!(
        s,
        "  async {method_name}({params_str}): Promise<{return_type}> {{"
    );

    // Build body if POST
    if ep.method == "POST" && (!required.is_empty() || has_optional) {
        let _ = writeln!(s, "    const body: Record<string, unknown> = {{}};");
        for (field_name, _field) in &required {
            let ts_name = ts_field(field_name);
            let _ = writeln!(s, "    body[\"{field_name}\"] = {ts_name};");
        }
        for (field_name, _field) in &optional {
            let _ = writeln!(
                s,
                "    if (options?.{field_name} !== undefined) body[\"{field_name}\"] = options.{field_name};",
            );
        }
    }

    // Auth token
    let auth_expr = match ep.auth.as_str() {
        "access_token" => "this.accessToken",
        "refresh_token" => "this.refreshToken",
        _ => "",
    };

    // Build request call
    let path_expr = ts_path_expr(ep);
    let has_body = ep.method == "POST" && (!required.is_empty() || has_optional);

    if auth_expr.is_empty() && !has_body {
        let _ = writeln!(
            s,
            "    const result = await this.request(\"{method}\", {path_expr});",
            method = ep.method,
        );
    } else {
        let mut opts_parts = Vec::new();
        if has_body {
            opts_parts.push("body".to_string());
        }
        if !auth_expr.is_empty() {
            opts_parts.push(format!("auth: {auth_expr}"));
        }
        let opts = opts_parts.join(", ");
        let _ = writeln!(
            s,
            "    const result = await this.request(\"{method}\", {path_expr}, {{ {opts} }});",
            method = ep.method,
        );
    }

    // Store tokens if the response includes them
    let resp_fields: Vec<&str> = ep.response.keys().map(|k| k.as_str()).collect();
    if resp_fields.contains(&"access_token") {
        let _ = writeln!(s, "    this.accessToken = result.access_token as string;");
    }
    if resp_fields.contains(&"refresh_token") {
        let _ = writeln!(s, "    this.refreshToken = result.refresh_token as string;");
    }

    // Return
    if has_response {
        let _ = writeln!(s, "    return result as {return_type};");
    }

    let _ = writeln!(s, "  }}");
}

// ─── src/index.ts ───────────────────────────────────────────────────────────

fn gen_index(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(1024);
    let _ = write!(s, "{}", file_header(&n.pascal));
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "export {{ {pascal}Client }} from \"./client\";",
        pascal = n.pascal
    );
    let _ = writeln!(
        s,
        "export {{ {pascal}Error }} from \"./errors\";",
        pascal = n.pascal
    );

    // Re-export error subclasses
    let mut error_classes: Vec<String> = Vec::new();
    for code in spec.error_codes.keys() {
        use heck::ToPascalCase;
        error_classes.push(format!("{}Error", code.to_pascal_case()));
    }
    if !error_classes.is_empty() {
        let _ = write!(s, "export {{ ");
        for (i, class) in error_classes.iter().enumerate() {
            if i > 0 {
                let _ = write!(s, ", ");
            }
            let _ = write!(s, "{class}");
        }
        let _ = writeln!(s, " }} from \"./errors\";");
    }

    // Re-export types
    let type_names: Vec<String> = spec
        .endpoints
        .iter()
        .filter(|(_, ep)| !ep.response.is_empty())
        .map(|(name, _)| ts_response_iface(name))
        .collect();
    if !type_names.is_empty() {
        let _ = write!(s, "export type {{ ");
        for (i, name) in type_names.iter().enumerate() {
            if i > 0 {
                let _ = write!(s, ", ");
            }
            let _ = write!(s, "{name}");
        }
        let _ = writeln!(s, " }} from \"./types\";");
    }

    GeneratedFile {
        path: format!("{}/src/index.ts", n.npm_name),
        content: s,
    }
}

// ─── package.json ───────────────────────────────────────────────────────────

fn gen_package_json(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let content = format!(
        r#"{{
  "name": "{npm_name}",
  "version": "{version}",
  "description": "{description}",
  "type": "module",
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "exports": {{
    ".": {{
      "import": "./dist/index.js",
      "types": "./dist/index.d.ts"
    }}
  }},
  "files": [
    "dist",
    "src"
  ],
  "scripts": {{
    "build": "tsc",
    "prepublishOnly": "npm run build"
  }},
  "devDependencies": {{
    "typescript": "^5.4"
  }},
  "engines": {{
    "node": ">=18"
  }},
  "license": "Apache-2.0",
  "repository": {{
    "type": "git",
    "url": "https://github.com/shroudb/{kebab}.git"
  }}
}}
"#,
        npm_name = n.npm_name,
        version = spec.api.version,
        description = n.description,
        kebab = n.kebab,
    );

    GeneratedFile {
        path: format!("{}/package.json", n.npm_name),
        content,
    }
}

// ─── tsconfig.json ──────────────────────────────────────────────────────────

fn gen_tsconfig(n: &Naming) -> GeneratedFile {
    let content = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "bundler",
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src"]
}
"#;

    GeneratedFile {
        path: format!("{}/tsconfig.json", n.npm_name),
        content: content.to_string(),
    }
}

// ─── README.md ──────────────────────────────────────────────────────────────

fn gen_readme(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(4096);

    let _ = writeln!(s, "# {npm_name}\n", npm_name = n.npm_name);
    let _ = writeln!(s, "{description}\n", description = n.description);
    let _ = writeln!(
        s,
        "> Auto-generated from the {raw} API spec. Do not edit.\n",
        raw = n.raw
    );

    let _ = writeln!(s, "## Installation\n");
    let _ = writeln!(
        s,
        "```bash\nnpm install {npm_name}\n```\n",
        npm_name = n.npm_name
    );

    let _ = writeln!(s, "## Quick Start\n");
    let _ = writeln!(s, "```typescript");
    let _ = writeln!(
        s,
        "import {{ {pascal}Client }} from \"{npm_name}\";\n",
        pascal = n.pascal,
        npm_name = n.npm_name,
    );
    let _ = writeln!(
        s,
        "const client = new {pascal}Client(\"http://localhost:{port}\");",
        pascal = n.pascal,
        port = n.default_port,
    );

    // Show first POST endpoint as example
    if let Some((ep_name, ep)) = spec.endpoints.iter().find(|(_, ep)| ep.method == "POST") {
        let method_name = ts_method(ep_name);
        let required = ep.required_body();
        let args: Vec<String> = required
            .iter()
            .map(|(name, field)| match field.field_type.as_str() {
                "string" => format!("\"example_{name}\""),
                "integer" => "1".to_string(),
                _ => "{}".to_string(),
            })
            .collect();
        let _ = writeln!(
            s,
            "const result = await client.{method_name}({args});",
            args = args.join(", "),
        );
        let _ = writeln!(s, "console.log(result);");
    }

    let _ = writeln!(s, "```\n");

    let _ = writeln!(s, "## API\n");
    let _ = writeln!(
        s,
        "### `new {pascal}Client(baseUrl: string, keyspace?: string)`\n",
        pascal = n.pascal,
    );
    let _ = writeln!(
        s,
        "Creates a new client. The `keyspace` parameter defaults to `\"default\"`.\n"
    );

    let _ = writeln!(s, "### Methods\n");
    for (ep_name, ep) in &spec.endpoints {
        let method_name = ts_method(ep_name);
        let required = ep.required_body();
        let params: Vec<String> = required
            .iter()
            .map(|(name, field)| {
                let ts_name = ts_field(name);
                let ty = ts_type(field);
                format!("{ts_name}: {ty}")
            })
            .collect();
        let optional = ep.optional_body();
        let has_opts = !optional.is_empty();
        let mut all_params = params.clone();
        if has_opts {
            use heck::ToPascalCase;
            all_params.push(format!("options?: {}Options", ep_name.to_pascal_case()));
        }
        let _ = writeln!(
            s,
            "- **`{method_name}({params})`** — {desc}",
            params = all_params.join(", "),
            desc = ep.description,
        );
    }
    let _ = writeln!(s);

    let _ = writeln!(s, "## License\n");
    let _ = writeln!(s, "Apache-2.0");

    GeneratedFile {
        path: format!("{}/README.md", n.npm_name),
        content: s,
    }
}
