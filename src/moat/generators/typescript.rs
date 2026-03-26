//! TypeScript unified SDK generator for Moat.
//!
//! Produces a single `@shroudb/sdk` package with engine-namespaced methods:
//!   client.vault.verify('ks', token)
//!   client.transit.encrypt('kr', plaintext)
//!   client.sentry.evaluate({...})
//!   client.control.createTenant({...})

use heck::{ToLowerCamelCase, ToPascalCase};

use super::MoatGenerator;
use crate::generator::GeneratedFile;
use crate::moat::spec::ResolvedMoatSpec;
use crate::wire::spec::ProtocolSpec;

pub struct TypeScriptMoatGenerator;

impl MoatGenerator for TypeScriptMoatGenerator {
    fn language(&self) -> &'static str {
        "TypeScript"
    }

    fn generate(&self, spec: &ResolvedMoatSpec) -> Vec<GeneratedFile> {
        let mut files = Vec::new();

        let pkg_name = spec
            .moat
            .sdk
            .as_ref()
            .and_then(|s| s.packages.as_ref())
            .and_then(|p| p.typescript.as_deref())
            .unwrap_or("@shroudb/sdk");

        // package.json
        files.push(GeneratedFile {
            path: "package.json".into(),
            content: generate_package_json(pkg_name, &spec.moat.protocol.version),
        });

        // tsconfig.json
        files.push(GeneratedFile {
            path: "tsconfig.json".into(),
            content: generate_tsconfig(),
        });

        // src/index.ts — main entry point
        files.push(GeneratedFile {
            path: "src/index.ts".into(),
            content: generate_index(spec),
        });

        // src/client.ts — ShrouDB unified client
        files.push(GeneratedFile {
            path: "src/client.ts".into(),
            content: generate_client(spec),
        });

        // src/transport.ts — HTTP transport
        files.push(GeneratedFile {
            path: "src/transport.ts".into(),
            content: generate_transport(),
        });

        // Per-engine namespace files
        for engine in &spec.moat.engines {
            if let Some(engine_spec) = spec.engine_specs.get(&engine.name) {
                files.push(GeneratedFile {
                    path: format!("src/{}.ts", engine.name),
                    content: generate_engine_namespace(&engine.name, engine_spec),
                });
            }
        }

        // src/control.ts — control plane methods
        if !spec.moat.control_plane.is_empty() {
            files.push(GeneratedFile {
                path: "src/control.ts".into(),
                content: generate_control_namespace(spec),
            });
        }

        files
    }
}

fn generate_package_json(name: &str, version: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "{version}",
  "description": "ShrouDB unified SDK — vault, transit, veil, sentry, mint, auth",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {{
    "build": "tsc",
    "prepublishOnly": "npm run build"
  }},
  "license": "MIT",
  "devDependencies": {{
    "typescript": "^5"
  }}
}}
"#
    )
}

fn generate_tsconfig() -> String {
    r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "Node16",
    "declaration": true,
    "outDir": "dist",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "moduleResolution": "Node16"
  },
  "include": ["src"]
}
"#
    .into()
}

fn generate_index(spec: &ResolvedMoatSpec) -> String {
    let mut out = String::new();
    out.push_str("export { ShrouDB, type ShrouDBOptions } from './client';\n");

    for engine in &spec.moat.engines {
        if spec.engine_specs.contains_key(&engine.name) {
            let pascal = engine.name.to_pascal_case();
            out.push_str(&format!(
                "export {{ {pascal}Namespace }} from './{}';\n",
                engine.name
            ));
        }
    }

    if !spec.moat.control_plane.is_empty() {
        out.push_str("export { ControlNamespace } from './control';\n");
    }

    out
}

fn generate_client(spec: &ResolvedMoatSpec) -> String {
    let mut out = String::new();
    out.push_str("import { HttpTransport } from './transport';\n");

    for engine in &spec.moat.engines {
        if spec.engine_specs.contains_key(&engine.name) {
            let pascal = engine.name.to_pascal_case();
            out.push_str(&format!(
                "import {{ {pascal}Namespace }} from './{}';\n",
                engine.name
            ));
        }
    }

    if !spec.moat.control_plane.is_empty() {
        out.push_str("import { ControlNamespace } from './control';\n");
    }

    out.push_str(
        r#"
export interface ShrouDBOptions {
  /** Moat HTTP endpoint (e.g. "https://moat.example.com") */
  endpoint: string;
  /** Bearer token for authentication */
  token?: string;
}

export class ShrouDB {
  private transport: HttpTransport;
"#,
    );

    // Engine namespace fields
    for engine in &spec.moat.engines {
        if spec.engine_specs.contains_key(&engine.name) {
            let pascal = engine.name.to_pascal_case();
            out.push_str(&format!(
                "  public readonly {}: {pascal}Namespace;\n",
                engine.name
            ));
        }
    }
    if !spec.moat.control_plane.is_empty() {
        out.push_str("  public readonly control: ControlNamespace;\n");
    }

    out.push_str(
        r#"
  constructor(options: ShrouDBOptions) {
    this.transport = new HttpTransport(options.endpoint, options.token);
"#,
    );

    for engine in &spec.moat.engines {
        if spec.engine_specs.contains_key(&engine.name) {
            let pascal = engine.name.to_pascal_case();
            let default_prefix = format!("/v1/{}", engine.name);
            let prefix = engine.http_prefix.as_deref().unwrap_or(&default_prefix);
            out.push_str(&format!(
                "    this.{} = new {pascal}Namespace(this.transport, '{prefix}');\n",
                engine.name
            ));
        }
    }
    if !spec.moat.control_plane.is_empty() {
        out.push_str("    this.control = new ControlNamespace(this.transport, '/v1/control');\n");
    }

    out.push_str("  }\n}\n");
    out
}

fn generate_transport() -> String {
    r#"/** Minimal HTTP transport for the ShrouDB SDK. */
export class HttpTransport {
  constructor(
    private baseUrl: string,
    private token?: string,
  ) {}

  async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }
    const resp = await fetch(url, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!resp.ok) {
      const text = await resp.text();
      throw new Error(`ShrouDB ${method} ${path}: ${resp.status} ${text}`);
    }
    return resp.json() as T;
  }

  async get<T>(path: string): Promise<T> {
    return this.request('GET', path);
  }

  async post<T>(path: string, body?: unknown): Promise<T> {
    return this.request('POST', path, body);
  }

  async delete<T>(path: string): Promise<T> {
    return this.request('DELETE', path);
  }
}
"#
    .into()
}

fn generate_engine_namespace(name: &str, spec: &ProtocolSpec) -> String {
    let pascal = name.to_pascal_case();
    let mut out = String::new();

    out.push_str("import { HttpTransport } from './transport';\n\n");
    out.push_str(&format!("export class {pascal}Namespace {{\n"));
    out.push_str("  constructor(\n");
    out.push_str("    private transport: HttpTransport,\n");
    out.push_str("    private prefix: string,\n");
    out.push_str("  ) {}\n\n");

    // Generate a method for each command.
    for (cmd_name, cmd_def) in &spec.commands {
        let method_name = cmd_name.to_lower_camel_case();
        let verb = &cmd_def.verb;

        // Determine HTTP method (POST for write ops, GET for reads).
        let http_method = match cmd_def.replica_behavior.as_str() {
            "PureRead" => "get",
            _ => "post",
        };

        // Build parameter list.
        let positional = cmd_def.positional_params();
        let named = cmd_def.named_params();

        let mut params = Vec::new();
        for p in &positional {
            let ts_type = spec
                .types
                .get(&p.param_type)
                .map(|t| t.typescript_type.as_str())
                .unwrap_or("string");
            params.push(format!("{}: {ts_type}", p.name));
        }
        if !named.is_empty() {
            let mut opts = Vec::new();
            for p in &named {
                let ts_type = spec
                    .types
                    .get(&p.param_type)
                    .map(|t| t.typescript_type.as_str())
                    .unwrap_or("string");
                opts.push(format!("    {}?: {ts_type};", p.name));
            }
            params.push(format!("options?: {{\n{}\n  }}", opts.join("\n")));
        }

        let param_str = params.join(", ");

        out.push_str(&format!("  /** {verb} — {} */\n", cmd_def.description));
        out.push_str(&format!(
            "  async {method_name}({param_str}): Promise<Record<string, unknown>> {{\n"
        ));

        // Build the request path. First positional param is usually the namespace (keyspace/keyring).
        if let Some(first) = positional.first() {
            out.push_str(&format!(
                "    return this.transport.{http_method}(`${{this.prefix}}/${{{}}}`, {});\n",
                first.name,
                if http_method == "post" {
                    format!(
                        "{{ verb: '{verb}'{} }}",
                        if positional.len() > 1 || !named.is_empty() {
                            ", ...options"
                        } else {
                            ""
                        }
                    )
                } else {
                    String::new()
                }
            ));
        } else {
            out.push_str(&format!(
                "    return this.transport.{http_method}(`${{this.prefix}}/{verb}`);\n"
            ));
        }

        out.push_str("  }\n\n");
    }

    out.push_str("}\n");
    out
}

fn generate_control_namespace(spec: &ResolvedMoatSpec) -> String {
    let mut out = String::new();
    out.push_str("import { HttpTransport } from './transport';\n\n");
    out.push_str("export class ControlNamespace {\n");
    out.push_str("  constructor(\n");
    out.push_str("    private transport: HttpTransport,\n");
    out.push_str("    private prefix: string,\n");
    out.push_str("  ) {}\n\n");

    for (name, endpoint) in &spec.moat.control_plane {
        let method_name = name.to_lower_camel_case();
        let http_method = endpoint.method.to_lowercase();

        // Build path (replace {id} and {tenant_id} with template literals).
        let ts_path = endpoint
            .path
            .replace("/v1/control", "${this.prefix}")
            .replace("{id}", "${id}")
            .replace("{tenant_id}", "${tenantId}");

        // Extract path params.
        let mut params = Vec::new();
        if endpoint.path.contains("{id}") {
            params.push("id: string".to_string());
        }
        if endpoint.path.contains("{tenant_id}") {
            params.push("tenantId: string".to_string());
        }
        if !endpoint.body.is_empty() {
            params.push("body: Record<string, unknown>".to_string());
        }

        let param_str = params.join(", ");

        out.push_str(&format!(
            "  /** {} {} — {} */\n",
            endpoint.method, endpoint.path, endpoint.description
        ));
        out.push_str(&format!(
            "  async {method_name}({param_str}): Promise<Record<string, unknown>> {{\n"
        ));
        out.push_str(&format!(
            "    return this.transport.{http_method}(`{ts_path}`{});\n",
            if !endpoint.body.is_empty() {
                ", body"
            } else {
                ""
            }
        ));
        out.push_str("  }\n\n");
    }

    out.push_str("}\n");
    out
}
