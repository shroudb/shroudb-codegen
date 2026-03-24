//! Ruby client generator.
//!
//! Produces a Ruby gem with:
//! - `lib/{snake}.rb`          — main require file
//! - `lib/{snake}/client.rb`   — HTTP client using `net/http`
//! - `lib/{snake}/errors.rb`   — error hierarchy
//! - `lib/{snake}/types.rb`    — Struct-based response types
//! - `{kebab}.gemspec`         — gem specification
//! - `README.md`               — quick usage docs

use super::super::spec::{ApiSpec, EndpointDef};
use crate::generator::{GeneratedFile, Naming};
use heck::{ToPascalCase, ToSnakeCase};
use std::fmt::Write;

use super::HttpGenerator;

pub struct RubyGenerator;

impl HttpGenerator for RubyGenerator {
    fn language(&self) -> &'static str {
        "Ruby"
    }

    fn generate(&self, spec: &ApiSpec, n: &Naming) -> Vec<GeneratedFile> {
        vec![
            gen_main_require(spec, n),
            gen_version(spec, n),
            gen_client(spec, n),
            gen_errors(spec, n),
            gen_types(spec, n),
            gen_gemspec(spec, n),
            gen_readme(spec, n),
        ]
    }
}

/// Map spec field types to Ruby type comment strings.
fn ruby_type(field_type: &str) -> &'static str {
    match field_type {
        "string" => "String",
        "integer" => "Integer",
        "json" => "Hash",
        "json_array" => "Array",
        _ => "Object",
    }
}

/// Build the Ruby method parameter list for an endpoint.
///
/// Required params come first as keyword arguments, then optional ones with `nil` defaults.
/// If the endpoint uses a keyspace, `keyspace:` is **not** added (it comes from the client).
fn method_params(ep: &EndpointDef) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (name, _field) in ep.required_body() {
        parts.push(format!("{}:", name));
    }
    for (name, _field) in ep.optional_body() {
        parts.push(format!("{}: nil", name));
    }
    parts.join(", ")
}

/// Build the Ruby path string for an endpoint, interpolating `{keyspace}` as `#{@keyspace}`.
fn ruby_path(ep: &EndpointDef) -> String {
    ep.path.replace("{keyspace}", r#"#{@keyspace}"#)
}

/// Determine whether the response contains `access_token` / `refresh_token` fields
/// so we can auto-store them on the client.
fn stores_tokens(ep: &EndpointDef) -> (bool, bool) {
    let has_access = ep.response.contains_key("access_token");
    let has_refresh = ep.response.contains_key("refresh_token");
    (has_access, has_refresh)
}

/// PascalCase response struct name for an endpoint key.
fn response_class(ep_name: &str) -> String {
    format!("{}Response", ep_name.to_pascal_case())
}

// ─── lib/{snake}.rb ─────────────────────────────────────────────────────────

fn gen_main_require(_spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    GeneratedFile {
        path: format!("lib/{}.rb", n.snake),
        content: format!(
            r#"# frozen_string_literal: true

# Auto-generated from {raw} API spec. Do not edit.

require_relative "{snake}/version"
require_relative "{snake}/errors"
require_relative "{snake}/types"
require_relative "{snake}/client"
"#,
            raw = n.raw,
            snake = n.snake,
        ),
    }
}

// ─── lib/{snake}/version.rb ─────────────────────────────────────────────────

fn gen_version(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    GeneratedFile {
        path: format!("lib/{}/version.rb", n.snake),
        content: format!(
            r#"# frozen_string_literal: true

# Auto-generated from {raw} API spec. Do not edit.

module {pascal}
  VERSION = "{version}"
end
"#,
            raw = n.raw,
            pascal = n.pascal,
            version = spec.api.version,
        ),
    }
}

// ─── lib/{snake}/client.rb ──────────────────────────────────────────────────

fn gen_client(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(4096);

    writeln!(s, "# frozen_string_literal: true").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "# Auto-generated from {} API spec. Do not edit.", n.raw).unwrap();
    writeln!(s).unwrap();
    writeln!(s, "require \"json\"").unwrap();
    writeln!(s, "require \"net/http\"").unwrap();
    writeln!(s, "require \"uri\"").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "module {}", n.pascal).unwrap();
    writeln!(s, "  class Client").unwrap();
    writeln!(s, "    attr_accessor :access_token, :refresh_token").unwrap();
    writeln!(s).unwrap();

    // Constructor
    writeln!(s, "    # Create a new {} client.", n.pascal).unwrap();
    writeln!(s, "    #").unwrap();
    writeln!(
        s,
        "    # @param base_url [String] Base URL of the {} server",
        n.raw
    )
    .unwrap();
    writeln!(
        s,
        "    # @param keyspace [String, nil] Default keyspace for endpoints that require one"
    )
    .unwrap();
    writeln!(s, "    def initialize(base_url:, keyspace: nil)").unwrap();
    writeln!(s, "      @base_url = base_url.chomp(\"/\")").unwrap();
    writeln!(s, "      @keyspace = keyspace").unwrap();
    writeln!(s, "      @access_token = nil").unwrap();
    writeln!(s, "      @refresh_token = nil").unwrap();
    writeln!(s, "    end").unwrap();

    // Generate a method for each endpoint
    for (ep_name, ep) in &spec.endpoints {
        writeln!(s).unwrap();
        let method_name = ep_name.to_snake_case();
        let params = method_params(ep);
        let resp_class = response_class(ep_name);
        let (stores_access, stores_refresh) = stores_tokens(ep);
        let path_str = ruby_path(ep);

        // Doc comment
        writeln!(s, "    # {}", ep.description).unwrap();
        for (name, field) in ep.required_body() {
            writeln!(
                s,
                "    # @param {} [{}] {}",
                name,
                ruby_type(&field.field_type),
                field.description
            )
            .unwrap();
        }
        for (name, field) in ep.optional_body() {
            writeln!(
                s,
                "    # @param {} [{}] {}",
                name,
                ruby_type(&field.field_type),
                field.description
            )
            .unwrap();
        }
        writeln!(s, "    # @return [{}]", resp_class).unwrap();

        // Method signature
        if params.is_empty() {
            writeln!(s, "    def {}", method_name).unwrap();
        } else {
            writeln!(s, "    def {}({})", method_name, params).unwrap();
        }

        // Keyspace guard
        if ep.has_keyspace() {
            writeln!(
                s,
                "      raise ArgumentError, \"keyspace is required\" if @keyspace.nil?"
            )
            .unwrap();
            writeln!(s).unwrap();
        }

        // Build body hash for requests with a body
        if ep.has_body() && !ep.body.is_empty() {
            writeln!(s, "      body = {{}}").unwrap();
            for (name, _field) in ep.required_body() {
                writeln!(s, "      body[\"{}\"] = {}", name, name).unwrap();
            }
            for (name, _field) in ep.optional_body() {
                writeln!(
                    s,
                    "      body[\"{}\"] = {} unless {}.nil?",
                    name, name, name
                )
                .unwrap();
            }
            writeln!(s).unwrap();
        }

        // Determine auth argument
        let auth_arg = match ep.auth.as_str() {
            "access_token" => ", auth: :access_token",
            "refresh_token" => ", auth: :refresh_token",
            "none" => "",
            other => panic!(
                "unknown auth type '{other}' on endpoint {}",
                ep_name.to_snake_case()
            ),
        };

        // Call request helper
        if ep.has_body() && !ep.body.is_empty() {
            writeln!(
                s,
                "      result = request(\"{}\", \"{}\", body: body{})",
                ep.method, path_str, auth_arg
            )
            .unwrap();
        } else {
            writeln!(
                s,
                "      result = request(\"{}\", \"{}\"{})",
                ep.method, path_str, auth_arg
            )
            .unwrap();
        }

        // Auto-store tokens (guarded with .key? check)
        if stores_access {
            writeln!(
                s,
                "      @access_token = result[\"access_token\"] if result.key?(\"access_token\")"
            )
            .unwrap();
        }
        if stores_refresh {
            writeln!(
                s,
                "      @refresh_token = result[\"refresh_token\"] if result.key?(\"refresh_token\")"
            )
            .unwrap();
        }

        // Return typed response
        writeln!(
            s,
            "      {}::{}(**result.transform_keys(&:to_sym))",
            n.pascal, resp_class
        )
        .unwrap();
        writeln!(s, "    end").unwrap();
    }

    // Private request helper
    writeln!(s).unwrap();
    writeln!(s, "    private").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "    def connection").unwrap();
    writeln!(s, "      @connection ||= begin").unwrap();
    writeln!(s, "        uri = URI.parse(@base_url)").unwrap();
    writeln!(s, "        http = Net::HTTP.new(uri.host, uri.port)").unwrap();
    writeln!(s, "        http.use_ssl = (uri.scheme == \"https\")").unwrap();
    writeln!(s, "        http.start").unwrap();
    writeln!(s, "        http").unwrap();
    writeln!(s, "      end").unwrap();
    writeln!(s, "    end").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "    def request(method, path, body: nil, auth: nil)").unwrap();
    writeln!(s, "      uri = URI.parse(\"#{{@base_url}}#{{path}}\")").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "      req = case method").unwrap();
    writeln!(
        s,
        "            when \"GET\"    then Net::HTTP::Get.new(uri)"
    )
    .unwrap();
    writeln!(
        s,
        "            when \"POST\"   then Net::HTTP::Post.new(uri)"
    )
    .unwrap();
    writeln!(
        s,
        "            when \"PUT\"    then Net::HTTP::Put.new(uri)"
    )
    .unwrap();
    writeln!(
        s,
        "            when \"PATCH\"  then Net::HTTP::Patch.new(uri)"
    )
    .unwrap();
    writeln!(
        s,
        "            when \"DELETE\" then Net::HTTP::Delete.new(uri)"
    )
    .unwrap();
    writeln!(
        s,
        "            else raise ArgumentError, \"unsupported method: #{{method}}\""
    )
    .unwrap();
    writeln!(s, "            end").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "      req[\"Content-Type\"] = \"application/json\"").unwrap();
    writeln!(s, "      req[\"Accept\"] = \"application/json\"").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "      case auth").unwrap();
    writeln!(s, "      when :access_token").unwrap();
    writeln!(
        s,
        "        raise Error.new(\"UNAUTHORIZED\", \"access_token is not set\") if @access_token.nil?"
    )
    .unwrap();
    writeln!(
        s,
        "        req[\"Authorization\"] = \"Bearer #{{@access_token}}\""
    )
    .unwrap();
    writeln!(s, "      when :refresh_token").unwrap();
    writeln!(
        s,
        "        raise Error.new(\"UNAUTHORIZED\", \"refresh_token is not set\") if @refresh_token.nil?"
    )
    .unwrap();
    writeln!(
        s,
        "        req[\"Authorization\"] = \"Bearer #{{@refresh_token}}\""
    )
    .unwrap();
    writeln!(s, "      end").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "      req.body = JSON.generate(body) if body").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "      response = connection.request(req)").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "      begin").unwrap();
    writeln!(
        s,
        "        data = response.body ? JSON.parse(response.body) : {{}}"
    )
    .unwrap();
    writeln!(s, "      rescue JSON::ParserError").unwrap();
    writeln!(s, "        raise Error.new(\"PARSE_ERROR\", \"invalid JSON in response: #{{response.body&.slice(0, 500)}}\")").unwrap();
    writeln!(s, "      end").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "      unless response.is_a?(Net::HTTPSuccess)").unwrap();
    writeln!(s, "        code = data[\"error\"] || \"UNKNOWN\"").unwrap();
    writeln!(s, "        detail = data[\"detail\"] || response.message").unwrap();
    writeln!(s, "        raise Error.from_code(code, detail)").unwrap();
    writeln!(s, "      end").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "      data").unwrap();
    writeln!(s, "    end").unwrap();
    writeln!(s, "  end").unwrap();
    writeln!(s, "end").unwrap();

    GeneratedFile {
        path: format!("lib/{}/client.rb", n.snake),
        content: s,
    }
}

// ─── lib/{snake}/errors.rb ──────────────────────────────────────────────────

fn gen_errors(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(2048);

    writeln!(s, "# frozen_string_literal: true").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "# Auto-generated from {} API spec. Do not edit.", n.raw).unwrap();
    writeln!(s).unwrap();
    writeln!(s, "module {}", n.pascal).unwrap();

    // Base error class
    writeln!(s, "  class Error < StandardError").unwrap();
    writeln!(s, "    attr_reader :code, :detail").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "    def initialize(code, detail = nil)").unwrap();
    writeln!(s, "      @code = code").unwrap();
    writeln!(s, "      @detail = detail").unwrap();
    writeln!(s, "      super(detail || code)").unwrap();
    writeln!(s, "    end").unwrap();
    writeln!(s).unwrap();

    // Factory method to instantiate the right subclass from an error code string
    writeln!(s, "    def self.from_code(code, detail = nil)").unwrap();
    writeln!(s, "      klass = ERROR_MAP[code] || Error").unwrap();
    writeln!(s, "      klass.new(code, detail)").unwrap();
    writeln!(s, "    end").unwrap();
    writeln!(s, "  end").unwrap();
    writeln!(s).unwrap();

    // Subclass per error code
    for (code, def) in &spec.error_codes {
        let class_name = format!("{}Error", code.to_pascal_case());
        writeln!(s, "  # {} (HTTP {})", def.description, def.http_status).unwrap();
        writeln!(s, "  class {} < Error; end", class_name).unwrap();
        writeln!(s).unwrap();
    }

    // Error map constant
    writeln!(s, "  ERROR_MAP = {{").unwrap();
    for code in spec.error_codes.keys() {
        let class_name = format!("{}Error", code.to_pascal_case());
        writeln!(s, "    \"{}\" => {},", code, class_name).unwrap();
    }
    writeln!(s, "  }}.freeze").unwrap();
    writeln!(s, "end").unwrap();

    GeneratedFile {
        path: format!("lib/{}/errors.rb", n.snake),
        content: s,
    }
}

// ─── lib/{snake}/types.rb ───────────────────────────────────────────────────

fn gen_types(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(2048);

    writeln!(s, "# frozen_string_literal: true").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "# Auto-generated from {} API spec. Do not edit.", n.raw).unwrap();
    writeln!(s).unwrap();
    writeln!(s, "module {}", n.pascal).unwrap();

    for (ep_name, ep) in &spec.endpoints {
        if ep.response.is_empty() {
            continue;
        }
        let class_name = response_class(ep_name);
        let required = ep.required_response();
        let optional = ep.optional_response();

        writeln!(
            s,
            "  # Response type for the {} endpoint.",
            ep_name.to_snake_case()
        )
        .unwrap();
        writeln!(s, "  # {}", ep.description).unwrap();

        // Collect all field names for the Struct
        let all_fields: Vec<&str> = required
            .iter()
            .chain(optional.iter())
            .map(|(name, _)| *name)
            .collect();

        let field_symbols: Vec<String> = all_fields.iter().map(|f| format!(":{}", f)).collect();

        writeln!(
            s,
            "  {} = Struct.new({}, keyword_init: true) do",
            class_name,
            field_symbols.join(", ")
        )
        .unwrap();

        // Add field documentation in a comment block
        for (name, field) in required.iter().chain(optional.iter()) {
            let opt_marker = if field.optional { " (optional)" } else { "" };
            writeln!(
                s,
                "    # @!attribute {} [{}{}] {}",
                name,
                ruby_type(&field.field_type),
                opt_marker,
                field.description
            )
            .unwrap();
        }

        writeln!(s, "  end").unwrap();
        writeln!(s).unwrap();
    }

    writeln!(s, "end").unwrap();

    GeneratedFile {
        path: format!("lib/{}/types.rb", n.snake),
        content: s,
    }
}

// ─── {kebab}.gemspec ────────────────────────────────────────────────────────

fn gen_gemspec(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(1024);

    writeln!(s, "# frozen_string_literal: true").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "# Auto-generated from {} API spec. Do not edit.", n.raw).unwrap();
    writeln!(s).unwrap();
    writeln!(s, "Gem::Specification.new do |spec|").unwrap();
    writeln!(s, "  spec.name          = \"{}\"", n.kebab).unwrap();
    writeln!(s, "  spec.version       = \"{}\"", spec.api.version).unwrap();
    writeln!(s, "  spec.summary       = \"{}\"", n.description).unwrap();
    writeln!(
        s,
        "  spec.description   = \"Ruby client for the {} HTTP API\"",
        n.pascal
    )
    .unwrap();
    writeln!(s, "  spec.license       = \"MIT\"").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "  spec.required_ruby_version = \">= 2.7.0\"").unwrap();
    writeln!(s).unwrap();
    writeln!(
        s,
        "  spec.files         = Dir[\"lib/**/*.rb\", \"README.md\", \"LICENSE\"]"
    )
    .unwrap();
    writeln!(s, "  spec.require_paths = [\"lib\"]").unwrap();
    writeln!(s, "end").unwrap();

    GeneratedFile {
        path: format!("{}.gemspec", n.kebab),
        content: s,
    }
}

// ─── README.md ──────────────────────────────────────────────────────────────

fn gen_readme(spec: &ApiSpec, n: &Naming) -> GeneratedFile {
    let mut s = String::with_capacity(2048);

    writeln!(s, "# {}", n.pascal).unwrap();
    writeln!(s).unwrap();
    writeln!(s, "Ruby client for the {} HTTP API.", n.pascal).unwrap();
    writeln!(s).unwrap();
    writeln!(
        s,
        "**Auto-generated from the {} API spec. Do not edit.**",
        n.raw
    )
    .unwrap();
    writeln!(s).unwrap();
    writeln!(s, "## Installation").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "Add to your `Gemfile`:").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "```ruby").unwrap();
    writeln!(s, "gem \"{}\"", n.kebab).unwrap();
    writeln!(s, "```").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "Or install directly:").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "```bash").unwrap();
    writeln!(s, "gem install {}", n.kebab).unwrap();
    writeln!(s, "```").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "## Usage").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "```ruby").unwrap();
    writeln!(s, "require \"{}\"", n.snake).unwrap();
    writeln!(s).unwrap();
    writeln!(
        s,
        "client = {}::Client.new(base_url: \"http://localhost:{}\", keyspace: \"default\")",
        n.pascal, n.default_port
    )
    .unwrap();
    writeln!(s).unwrap();

    // Show example for first POST endpoint with body (likely signup or login)
    let mut shown_example = false;
    for (ep_name, ep) in &spec.endpoints {
        if ep.method != "POST" || ep.body.is_empty() || shown_example {
            continue;
        }
        let method_name = ep_name.to_snake_case();
        let mut args: Vec<String> = Vec::new();
        for (name, _field) in ep.required_body() {
            let example_val = match name {
                "user_id" => "\"alice\"",
                "password" => "\"s3cret\"",
                _ => "\"...\"",
            };
            args.push(format!("{}: {}", name, example_val));
        }
        writeln!(s, "result = client.{}({})", method_name, args.join(", ")).unwrap();
        writeln!(s, "puts result.access_token").unwrap();
        shown_example = true;
    }

    writeln!(s, "```").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "## Available Methods").unwrap();
    writeln!(s).unwrap();
    for (ep_name, ep) in &spec.endpoints {
        let method_name = ep_name.to_snake_case();
        writeln!(s, "- `client.{}` -- {}", method_name, ep.description).unwrap();
    }
    writeln!(s).unwrap();
    writeln!(s, "## Error Handling").unwrap();
    writeln!(s).unwrap();
    writeln!(
        s,
        "All API errors raise `{}::Error` (or a specific subclass):",
        n.pascal
    )
    .unwrap();
    writeln!(s).unwrap();
    writeln!(s, "```ruby").unwrap();
    writeln!(s, "begin").unwrap();
    writeln!(s, "  client.login(user_id: \"alice\", password: \"wrong\")").unwrap();
    writeln!(s, "rescue {}::Error => e", n.pascal).unwrap();
    writeln!(s, "  puts e.code   # => \"UNAUTHORIZED\"").unwrap();
    writeln!(s, "  puts e.detail # => \"invalid credentials\"").unwrap();
    writeln!(s, "end").unwrap();
    writeln!(s, "```").unwrap();

    GeneratedFile {
        path: "README.md".to_string(),
        content: s,
    }
}
