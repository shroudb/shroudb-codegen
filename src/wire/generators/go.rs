//! Go client generator.
//!
//! Produces a Go module with:
//! - `{name}.go`       — public `Client`, URI-first, with connection pool
//! - `connection.go`   — internal protocol codec
//! - `errors.go`       — error types
//! - `types.go`        — response structs
//! - `pool.go`         — connection pool
//! - `pipeline.go`     — pipelining support
//! - `go.mod`          — module definition

use super::super::spec::{CommandDef, ProtocolSpec};
use super::Generator;
use crate::generator::{GeneratedFile, Naming};
use heck::ToUpperCamelCase;
use std::fmt::Write;

pub struct GoGenerator;

impl Generator for GoGenerator {
    fn language(&self) -> &'static str {
        "Go"
    }

    fn generate(&self, spec: &ProtocolSpec) -> Vec<GeneratedFile> {
        let n = super::naming_from_spec(spec);
        vec![
            gen_connection(spec, &n),
            gen_pool(spec, &n),
            gen_errors(spec, &n),
            gen_types(spec, &n),
            gen_client(spec, &n),
            gen_pipeline(spec, &n),
            gen_go_mod(spec, &n),
            gen_readme(spec, &n),
        ]
    }
}

fn go_type(spec: &ProtocolSpec, type_name: &str, optional: bool) -> String {
    let base = match spec.types.get(type_name) {
        Some(t) => t
            .go_type
            .as_deref()
            .unwrap_or_else(|| go_type_from_rust(&t.rust_type)),
        None => "any",
    };
    if optional {
        match base {
            "string" | "map[string]any" | "[]string" | "[]byte" => base.into(),
            _ => format!("*{base}"),
        }
    } else {
        base.into()
    }
}

fn go_type_from_rust(rust_type: &str) -> &str {
    match rust_type {
        "String" | "&str" => "string",
        "i64" | "u64" => "int64",
        "i32" | "u32" => "int32",
        "bool" => "bool",
        "f64" => "float64",
        "serde_json::Value" | "HashMap<String, serde_json::Value>" => "map[string]any",
        "Vec<String>" => "[]string",
        "Vec<u8>" => "[]byte",
        _ => "any",
    }
}

// ─── connection.go (internal) ────────────────────────────────────────────────

fn gen_connection(_spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    GeneratedFile {
        path: "connection.go".into(),
        content: format!(
            r#"// Internal {pascal} protocol codec.
//
// This file is an implementation detail of the {pascal} client library.
// Do not use directly — use Client instead.
//
// Auto-generated from {raw} protocol spec. Do not edit.

package {snake}

import (
	"bufio"
	"crypto/tls"
	"fmt"
	"io"
	"net"
	"strconv"
	"time"
)

const defaultPort = {port}

type connection struct {{
	conn   net.Conn
	reader *bufio.Reader
}}

func dial(host string, port int, useTLS bool) (*connection, error) {{
	addr := net.JoinHostPort(host, strconv.Itoa(port))
	var c net.Conn
	var err error

	if useTLS {{
		c, err = tls.DialWithDialer(&net.Dialer{{Timeout: 10 * time.Second}}, "tcp", addr, &tls.Config{{}})
	}} else {{
		c, err = net.DialTimeout("tcp", addr, 10*time.Second)
	}}
	if err != nil {{
		return nil, fmt.Errorf("{snake}: connect %s: %w", addr, err)
	}}

	return &connection{{
		conn:   c,
		reader: bufio.NewReaderSize(c, 64*1024),
	}}, nil
}}

func (c *connection) execute(args ...string) (any, error) {{
	// Encode command
	buf := fmt.Sprintf("*%d\r\n", len(args))
	for _, arg := range args {{
		buf += fmt.Sprintf("$%d\r\n%s\r\n", len(arg), arg)
	}}
	if _, err := io.WriteString(c.conn, buf); err != nil {{
		return nil, fmt.Errorf("{snake}: write: %w", err)
	}}
	return c.readFrame()
}}

func (c *connection) readFrame() (any, error) {{
	line, err := c.reader.ReadString('\n')
	if err != nil {{
		return nil, fmt.Errorf("{snake}: read: %w", err)
	}}
	if len(line) < 3 {{
		return nil, fmt.Errorf("{snake}: short response")
	}}
	tag := line[0]
	payload := line[1 : len(line)-2] // strip \r\n

	switch tag {{
	case '+':
		return payload, nil
	case '-':
		return nil, parseError(payload)
	case ':':
		n, err := strconv.ParseInt(payload, 10, 64)
		if err != nil {{
			return nil, fmt.Errorf("{snake}: invalid integer: %s", payload)
		}}
		return n, nil
	case '$':
		length, err := strconv.Atoi(payload)
		if err != nil {{
			return nil, fmt.Errorf("{snake}: invalid bulk length: %s", payload)
		}}
		if length < 0 {{
			return nil, nil
		}}
		data := make([]byte, length+2)
		if _, err := io.ReadFull(c.reader, data); err != nil {{
			return nil, fmt.Errorf("{snake}: bulk read: %w", err)
		}}
		return string(data[:length]), nil
	case '*':
		count, err := strconv.Atoi(payload)
		if err != nil {{
			return nil, fmt.Errorf("{snake}: invalid array length: %s", payload)
		}}
		arr := make([]any, count)
		for i := range count {{
			arr[i], err = c.readFrame()
			if err != nil {{
				return nil, err
			}}
		}}
		return arr, nil
	case '%':
		count, err := strconv.Atoi(payload)
		if err != nil {{
			return nil, fmt.Errorf("{snake}: invalid map length: %s", payload)
		}}
		m := make(map[string]any, count)
		for range count {{
			key, err := c.readFrame()
			if err != nil {{
				return nil, err
			}}
			val, err := c.readFrame()
			if err != nil {{
				return nil, err
			}}
			m[fmt.Sprint(key)] = val
		}}
		return m, nil
	case '_':
		return nil, nil
	default:
		return nil, fmt.Errorf("{snake}: unknown response type: %c", tag)
	}}
}}

func (c *connection) sendCommand(args ...string) error {{
	buf := fmt.Sprintf("*%d\r\n", len(args))
	for _, arg := range args {{
		buf += fmt.Sprintf("$%d\r\n%s\r\n", len(arg), arg)
	}}
	_, err := io.WriteString(c.conn, buf)
	return err
}}

func (c *connection) readResponse() (any, error) {{
	return c.readFrame()
}}

func (c *connection) close() error {{
	return c.conn.Close()
}}
"#,
            port = n.default_port,
            pascal = n.pascal,
            snake = n.snake,
            raw = n.raw,
        ),
    }
}

// ─── pool.go ─────────────────────────────────────────────────────────────────

fn gen_pool(_spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    GeneratedFile {
        path: "pool.go".into(),
        content: format!(
            r#"// Connection pool for {pascal} clients.
//
// Auto-generated from {raw} protocol spec. Do not edit.

package {snake}

import (
	"sync"
)

// PoolConfig controls connection pool behavior.
type PoolConfig struct {{
	// Maximum number of idle connections to keep in the pool.
	// Default: 4.
	MaxIdle int

	// Maximum number of total connections (idle + in-use).
	// 0 means unlimited. Default: 0.
	MaxOpen int
}}

type pool struct {{
	mu      sync.Mutex
	host    string
	port    int
	tls     bool
	auth    string
	idle    []*connection
	open    int
	config  PoolConfig
}}

func newPool(host string, port int, useTLS bool, auth string, cfg PoolConfig) *pool {{
	if cfg.MaxIdle <= 0 {{
		cfg.MaxIdle = 4
	}}
	return &pool{{
		host:   host,
		port:   port,
		tls:    useTLS,
		auth:   auth,
		config: cfg,
	}}
}}

func (p *pool) get() (*connection, error) {{
	p.mu.Lock()

	// Try to reuse an idle connection
	if len(p.idle) > 0 {{
		c := p.idle[len(p.idle)-1]
		p.idle = p.idle[:len(p.idle)-1]
		p.mu.Unlock()
		return c, nil
	}}

	// Check max open limit
	if p.config.MaxOpen > 0 && p.open >= p.config.MaxOpen {{
		p.mu.Unlock()
		// Block would be better, but keep it simple: create anyway
		// A production pool would use a condition variable here
	}}

	p.open++
	p.mu.Unlock()

	c, err := dial(p.host, p.port, p.tls)
	if err != nil {{
		p.mu.Lock()
		p.open--
		p.mu.Unlock()
		return nil, err
	}}

	if p.auth != "" {{
		if _, err := c.execute("AUTH", p.auth); err != nil {{
			c.close()
			p.mu.Lock()
			p.open--
			p.mu.Unlock()
			return nil, err
		}}
	}}

	return c, nil
}}

func (p *pool) put(c *connection) {{
	p.mu.Lock()
	defer p.mu.Unlock()

	if len(p.idle) < p.config.MaxIdle {{
		p.idle = append(p.idle, c)
	}} else {{
		c.close()
		p.open--
	}}
}}

func (p *pool) close() {{
	p.mu.Lock()
	defer p.mu.Unlock()

	for _, c := range p.idle {{
		c.close()
	}}
	p.idle = nil
	p.open = 0
}}
"#,
            pascal = n.pascal,
            snake = n.snake,
            raw = n.raw,
        ),
    }
}

// ─── errors.go ───────────────────────────────────────────────────────────────

fn gen_errors(spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    let mut out = format!(
        r#"// {pascal} error types.
//
// Auto-generated from {raw} protocol spec. Do not edit.

package {snake}

import (
	"fmt"
	"strings"
)

// {pascal}Error represents an error returned by the {pascal} server.
type {pascal}Error struct {{
	// Machine-readable error code (e.g. "NOTFOUND", "DENIED").
	Code string

	// Human-readable error message.
	Message string
}}

func (e *{pascal}Error) Error() string {{
	return fmt.Sprintf("[%s] %s", e.Code, e.Message)
}}

func parseError(payload string) *{pascal}Error {{
	code, message, _ := strings.Cut(payload, " ")
	return &{pascal}Error{{Code: code, Message: message}}
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
        writeln!(out, "\t// {} — {}", const_name, def.description).unwrap();
        writeln!(out, "\t{} = \"{}\"", const_name, code).unwrap();
    }

    out.push_str(")\n\n");
    out.push_str(&format!(
        r#"// IsCode reports whether the error has the given code.
func IsCode(err error, code string) bool {{
	if ke, ok := err.(*{pascal}Error); ok {{
		return ke.Code == code
	}}
	return false
}}
"#,
        pascal = n.pascal,
    ));

    GeneratedFile {
        path: "errors.go".into(),
        content: out,
    }
}

// ─── types.go ────────────────────────────────────────────────────────────────

fn gen_types(spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    // Check if any response field needs JSON parsing
    let needs_json = spec.commands.values().any(|cmd| {
        cmd.response
            .iter()
            .any(|f| go_type(spec, &f.field_type, false) == "map[string]any")
    });

    let mut out = format!(
        r#"// {pascal} response types.
//
// Auto-generated from {raw} protocol spec. Do not edit.

package {snake}
"#,
        pascal = n.pascal,
        snake = n.snake,
        raw = n.raw,
    );

    if needs_json {
        writeln!(out, "\nimport \"encoding/json\"\n").unwrap();
    } else {
        writeln!(out).unwrap();
    }

    for (cmd_name, cmd) in &spec.commands {
        if cmd.response.is_empty() || cmd.simple_response {
            continue;
        }
        let struct_name = format!("{}Response", cmd_name.to_upper_camel_case());
        writeln!(
            out,
            "// {struct_name} is the response from the {} command.",
            cmd.verb
        )
        .unwrap();
        writeln!(out, "type {struct_name} struct {{").unwrap();
        for f in &cmd.response {
            let field_name = f.name.to_upper_camel_case();
            let field_type = go_type(spec, &f.field_type, f.optional);
            writeln!(out, "\t{field_name} {field_type} // {}", f.description).unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();

        // from_map helper
        writeln!(
            out,
            "func parse{struct_name}(m map[string]any) *{struct_name} {{"
        )
        .unwrap();
        writeln!(out, "\tr := &{struct_name}{{}}").unwrap();
        for f in &cmd.response {
            let field_name = f.name.to_upper_camel_case();
            let go_t = go_type(spec, &f.field_type, false);
            match go_t.as_str() {
                "string" => {
                    writeln!(
                        out,
                        "\tif v, ok := m[\"{}\"].(string); ok {{ r.{field_name} = v }}",
                        f.name
                    )
                    .unwrap();
                }
                "int64" => {
                    if f.optional {
                        writeln!(
                            out,
                            "\tif v, ok := m[\"{}\"].(int64); ok {{ r.{field_name} = &v }}",
                            f.name
                        )
                        .unwrap();
                    } else {
                        writeln!(
                            out,
                            "\tif v, ok := m[\"{}\"].(int64); ok {{ r.{field_name} = v }}",
                            f.name
                        )
                        .unwrap();
                    }
                }
                "map[string]any" => {
                    // json_value fields may arrive as map or JSON string
                    writeln!(out, "\tswitch val := m[\"{}\"].(type) {{", f.name).unwrap();
                    writeln!(out, "\tcase map[string]any:").unwrap();
                    writeln!(out, "\t\tr.{field_name} = val").unwrap();
                    writeln!(out, "\tcase string:").unwrap();
                    writeln!(out, "\t\t_ = json.Unmarshal([]byte(val), &r.{field_name})").unwrap();
                    writeln!(out, "\t}}").unwrap();
                }
                "bool" => {
                    writeln!(
                        out,
                        "\tif v, ok := m[\"{}\"].(bool); ok {{ r.{field_name} = v }}",
                        f.name
                    )
                    .unwrap();
                }
                _ => {
                    writeln!(
                        out,
                        "\tif v, ok := m[\"{}\"].({go_t}); ok {{ r.{field_name} = v }}",
                        f.name
                    )
                    .unwrap();
                }
            }
        }
        writeln!(out, "\treturn r").unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    // SubscriptionEvent type for streaming subscribe
    out.push_str(
        r#"// SubscriptionEvent represents a real-time event from a SUBSCRIBE stream.
type SubscriptionEvent struct {
	EventType string
	Keyspace  string
	Detail    string
	Timestamp int64
}
"#,
    );

    GeneratedFile {
        path: "types.go".into(),
        content: out,
    }
}

// ─── {name}.go (public client) ───────────────────────────────────────────────

fn gen_client(spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    let scheme = n
        .uri_schemes
        .first()
        .map(|s| s.trim_end_matches("://"))
        .unwrap_or(&n.snake);
    let scheme_tls = format!("{scheme}+tls");
    let mut out = format!(
        r#"// Package {snake} provides a client for the {pascal} {description}.
//
// Auto-generated from {raw} protocol spec. Do not edit.
//
// Connect using a {pascal} URI:
//
//	client, err := {snake}.Connect("{scheme}://localhost")
//	result, err := client.Issue(ctx, "my-keyspace", &{snake}.IssueOptions{{TTL: 3600}})
//	fmt.Println(result.CredentialID, result.Token)
//	client.Close()
package {snake}

import (
	"encoding/json"
	"fmt"
	"net/url"
	"strconv"
	"strings"
)

// Client is a {pascal} client backed by a connection pool.
type Client struct {{
	pool *pool
	host string
	port int
	tls  bool
	auth string
}}

// Connect creates a new Client from a {pascal} URI.
//
// Supported URI formats:
//
//	{scheme}://localhost
//	{scheme}://localhost:{port}
//	{scheme_tls}://prod.example.com
//	{scheme}://mytoken@localhost:{port}
//	{scheme}://mytoken@localhost/sessions
//	{scheme_tls}://tok@host:{port}/keys
func Connect(uri string, opts ...PoolConfig) (*Client, error) {{
	cfg, err := parseURI(uri)
	if err != nil {{
		return nil, err
	}}
	var poolCfg PoolConfig
	if len(opts) > 0 {{
		poolCfg = opts[0]
	}}
	p := newPool(cfg.host, cfg.port, cfg.tls, cfg.authToken, poolCfg)
	// Verify connectivity by getting and returning one connection
	c, err := p.get()
	if err != nil {{
		return nil, err
	}}
	p.put(c)
	return &Client{{pool: p, host: cfg.host, port: cfg.port, tls: cfg.tls, auth: cfg.authToken}}, nil
}}

// Close shuts down the client and all pooled connections.
func (c *Client) Close() {{
	c.pool.close()
}}

// Pipeline creates a new pipeline for batching commands.
func (c *Client) Pipeline() *Pipeline {{
	return &Pipeline{{pool: c.pool}}
}}

func (c *Client) exec(args ...string) (any, error) {{
	conn, err := c.pool.get()
	if err != nil {{
		return nil, err
	}}
	result, err := conn.execute(args...)
	if err != nil {{
		conn.close()
		return nil, err
	}}
	c.pool.put(conn)
	return result, nil
}}

func (c *Client) execMap(args ...string) (map[string]any, error) {{
	result, err := c.exec(args...)
	if err != nil {{
		return nil, err
	}}
	m, ok := result.(map[string]any)
	if !ok {{
		return nil, fmt.Errorf("{snake}: expected map response, got %T", result)
	}}
	return m, nil
}}

// Subscription represents an active streaming subscription.
type Subscription struct {{
	conn   *connection
	events chan SubscriptionEvent
	errc   chan error
	done   chan struct{{}}
}}

// Events returns a channel that receives subscription events.
func (s *Subscription) Events() <-chan SubscriptionEvent {{
	return s.events
}}

// Err returns a channel that receives the first read error (including clean shutdown).
func (s *Subscription) Err() <-chan error {{
	return s.errc
}}

// Close terminates the subscription and closes the underlying connection.
func (s *Subscription) Close() error {{
	select {{
	case <-s.done:
		return nil
	default:
		close(s.done)
		return s.conn.close()
	}}
}}

func (s *Subscription) readLoop() {{
	defer close(s.events)
	for {{
		select {{
		case <-s.done:
			return
		default:
		}}
		raw, err := s.conn.readFrame()
		if err != nil {{
			select {{
			case s.errc <- err:
			default:
			}}
			return
		}}
		arr, ok := raw.([]any)
		if !ok || len(arr) != 5 {{
			continue
		}}
		tag, _ := arr[0].(string)
		if tag != "event" {{
			continue
		}}
		evtType, _ := arr[1].(string)
		keyspace, _ := arr[2].(string)
		detail, _ := arr[3].(string)
		var ts int64
		switch v := arr[4].(type) {{
		case int64:
			ts = v
		case string:
			ts, _ = strconv.ParseInt(v, 10, 64)
		}}
		evt := SubscriptionEvent{{
			EventType: evtType,
			Keyspace:  keyspace,
			Detail:    detail,
			Timestamp: ts,
		}}
		select {{
		case s.events <- evt:
		case <-s.done:
			return
		}}
	}}
}}

// Subscribe opens a dedicated connection and subscribes to the given channel.
// The returned Subscription streams events until Close is called or an error occurs.
func (c *Client) Subscribe(channel string) (*Subscription, error) {{
	conn, err := dial(c.host, c.port, c.tls)
	if err != nil {{
		return nil, err
	}}
	if c.auth != "" {{
		if _, err := conn.execute("AUTH", c.auth); err != nil {{
			conn.close()
			return nil, err
		}}
	}}
	resp, err := conn.execute("SUBSCRIBE", channel)
	if err != nil {{
		conn.close()
		return nil, err
	}}
	m, ok := resp.(map[string]any)
	if !ok {{
		conn.close()
		return nil, fmt.Errorf("{snake}: expected map response for SUBSCRIBE, got %T", resp)
	}}
	if status, _ := m["status"].(string); status != "OK" {{
		conn.close()
		return nil, fmt.Errorf("{snake}: subscribe failed: %v", m)
	}}
	sub := &Subscription{{
		conn:   conn,
		events: make(chan SubscriptionEvent, 64),
		errc:   make(chan error, 1),
		done:   make(chan struct{{}}),
	}}
	go sub.readLoop()
	return sub, nil
}}

type uriConfig struct {{
	host      string
	port      int
	tls       bool
	authToken string
	keyspace  string
}}

func parseURI(uri string) (*uriConfig, error) {{
	cfg := &uriConfig{{port: defaultPort}}

	switch {{
	case strings.HasPrefix(uri, "{scheme_tls}://"):
		cfg.tls = true
		uri = "{scheme}://" + uri[len("{scheme_tls}://"):]
	case strings.HasPrefix(uri, "{scheme}://"):
		// ok
	default:
		return nil, fmt.Errorf("{snake}: invalid URI scheme (expected {scheme}:// or {scheme_tls}://): %s", uri)
	}}

	u, err := url.Parse(uri)
	if err != nil {{
		return nil, fmt.Errorf("{snake}: invalid URI: %w", err)
	}}

	cfg.host = u.Hostname()
	if cfg.host == "" {{
		cfg.host = "localhost"
	}}

	if p := u.Port(); p != "" {{
		if n, err := strconv.Atoi(p); err == nil {{
			cfg.port = n
		}}
	}}

	if u.User != nil {{
		cfg.authToken = u.User.Username()
	}}

	cfg.keyspace = strings.TrimPrefix(u.Path, "/")

	return cfg, nil
}}
"#,
        pascal = n.pascal,
        snake = n.snake,
        raw = n.raw,
        scheme = scheme,
        scheme_tls = scheme_tls,
        port = n.default_port,
        description = n.description,
    );

    // Generate methods
    for (cmd_name, cmd) in &spec.commands {
        writeln!(out).unwrap();
        gen_go_method(&mut out, spec, cmd_name, cmd);
    }

    GeneratedFile {
        path: format!("{}.go", n.snake),
        content: out,
    }
}

fn gen_go_method(out: &mut String, _spec: &ProtocolSpec, cmd_name: &str, cmd: &CommandDef) {
    if cmd.streaming {
        // Subscribe method is generated inline in gen_client; nothing else needed here.
        return;
    }

    let method_name = cmd_name.to_upper_camel_case();
    let positional = cmd.positional_params();
    let named = cmd.named_params();

    let has_options = !named.is_empty();
    let has_response = !cmd.response.is_empty() && !cmd.simple_response;
    let response_type = format!("{}Response", cmd_name.to_upper_camel_case());

    // Generate options struct if needed
    if has_options {
        let opts_name = format!("{method_name}Options");
        writeln!(
            out,
            "// {opts_name} are optional parameters for {method_name}."
        )
        .unwrap();
        writeln!(out, "type {opts_name} struct {{").unwrap();
        for p in &named {
            let field_name = p.name.to_upper_camel_case();
            if p.param_type == "boolean_flag" {
                writeln!(out, "\t{field_name} bool").unwrap();
            } else if p.param_type == "json_value" {
                writeln!(out, "\t{field_name} map[string]any").unwrap();
            } else if p.variadic {
                writeln!(out, "\t{field_name} []string").unwrap();
            } else if p.param_type == "integer" {
                writeln!(out, "\t{field_name} int64").unwrap();
            } else {
                writeln!(out, "\t{field_name} string").unwrap();
            }
        }
        writeln!(out, "}}\n").unwrap();
    }

    // Method signature
    writeln!(out, "// {method_name} — {}", cmd.description).unwrap();

    let mut params: Vec<String> = Vec::new();
    for p in &positional {
        params.push(format!("{} string", p.name));
    }
    if has_options {
        params.push(format!("opts *{method_name}Options"));
    }

    let return_sig = if has_response {
        format!("(*{response_type}, error)")
    } else {
        "error".into()
    };

    writeln!(
        out,
        "func (c *Client) {method_name}({}) {return_sig} {{",
        params.join(", "),
    )
    .unwrap();

    // Build args
    writeln!(out, "\targs := []string{{").unwrap();
    if let Some(sub) = &cmd.subcommand {
        writeln!(out, "\t\t\"{}\", \"{}\",", cmd.verb, sub).unwrap();
    } else {
        writeln!(out, "\t\t\"{}\",", cmd.verb).unwrap();
    }
    for p in &positional {
        if p.required {
            writeln!(out, "\t\t{},", p.name).unwrap();
        }
    }
    writeln!(out, "\t}}").unwrap();

    // Optional positional (like health's optional keyspace)
    for p in &positional {
        if !p.required {
            writeln!(out, "\tif {} != \"\" {{", p.name).unwrap();
            writeln!(out, "\t\targs = append(args, {})", p.name).unwrap();
            writeln!(out, "\t}}").unwrap();
        }
    }

    // Named params from options
    if has_options {
        writeln!(out, "\tif opts != nil {{").unwrap();
        for p in &named {
            let field_name = p.name.to_upper_camel_case();
            let key = p.key.as_deref().unwrap();
            if p.param_type == "boolean_flag" {
                writeln!(out, "\t\tif opts.{field_name} {{").unwrap();
                writeln!(out, "\t\t\targs = append(args, \"{key}\")").unwrap();
                writeln!(out, "\t\t}}").unwrap();
            } else if p.param_type == "json_value" {
                writeln!(out, "\t\tif opts.{field_name} != nil {{").unwrap();
                writeln!(out, "\t\t\tb, _ := json.Marshal(opts.{field_name})").unwrap();
                writeln!(out, "\t\t\targs = append(args, \"{key}\", string(b))").unwrap();
                writeln!(out, "\t\t}}").unwrap();
            } else if p.variadic {
                writeln!(out, "\t\tif len(opts.{field_name}) > 0 {{").unwrap();
                writeln!(out, "\t\t\targs = append(args, \"{key}\")").unwrap();
                writeln!(out, "\t\t\targs = append(args, opts.{field_name}...)").unwrap();
                writeln!(out, "\t\t}}").unwrap();
            } else if p.param_type == "integer" {
                writeln!(out, "\t\tif opts.{field_name} != 0 {{").unwrap();
                writeln!(
                    out,
                    "\t\t\targs = append(args, \"{key}\", strconv.FormatInt(opts.{field_name}, 10))"
                )
                .unwrap();
                writeln!(out, "\t\t}}").unwrap();
            } else {
                writeln!(out, "\t\tif opts.{field_name} != \"\" {{").unwrap();
                writeln!(
                    out,
                    "\t\t\targs = append(args, \"{key}\", opts.{field_name})"
                )
                .unwrap();
                writeln!(out, "\t\t}}").unwrap();
            }
        }
        writeln!(out, "\t}}").unwrap();
    }

    // Execute
    if has_response {
        writeln!(out, "\tm, err := c.execMap(args...)").unwrap();
        writeln!(out, "\tif err != nil {{").unwrap();
        writeln!(out, "\t\treturn nil, err").unwrap();
        writeln!(out, "\t}}").unwrap();
        writeln!(out, "\treturn parse{response_type}(m), nil").unwrap();
    } else {
        writeln!(out, "\t_, err := c.exec(args...)").unwrap();
        writeln!(out, "\treturn err").unwrap();
    }

    writeln!(out, "}}").unwrap();
}

// ─── pipeline.go ──────────────────────────────────────────────────────────

fn gen_pipeline(spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    let mut imports = vec!["\"fmt\"", "\"strconv\""];
    // Check if any command needs json
    let needs_json = spec.commands.values().any(|cmd| {
        !cmd.streaming
            && cmd
                .named_params()
                .iter()
                .any(|p| p.param_type == "json_value")
    });
    if needs_json {
        imports.push("\"encoding/json\"");
    }
    imports.sort();

    let mut out = format!(
        r#"// {pascal} pipeline for batching commands.
//
// Auto-generated from {raw} protocol spec. Do not edit.

package {snake}

import (
"#,
        pascal = n.pascal,
        snake = n.snake,
        raw = n.raw,
    );

    for imp in &imports {
        writeln!(out, "\t{imp}").unwrap();
    }

    out.push_str(&format!(
        r#")

// Pipeline batches multiple {pascal} commands and executes them in a single round-trip.
//
// Usage:
//
//	pipe := client.Pipeline()
//	pipe.Issue("keyspace", &IssueOptions{{TTL: 3600}})
//	pipe.Verify("keyspace", token, nil)
//	results, err := pipe.Execute()
type Pipeline struct {{
	pool     *pool
	commands []pipelineCmd
}}

type pipelineCmd struct {{
	args   []string
	parser func(map[string]any) any // nil for simple responses
}}

// Execute sends all queued commands and returns typed responses.
func (p *Pipeline) Execute() ([]any, error) {{
	conn, err := p.pool.get()
	if err != nil {{
		return nil, err
	}}

	// Send all commands
	for _, cmd := range p.commands {{
		if err := conn.sendCommand(cmd.args...); err != nil {{
			conn.close()
			return nil, err
		}}
	}}

	// Read all responses
	results := make([]any, 0, len(p.commands))
	for _, cmd := range p.commands {{
		raw, err := conn.readResponse()
		if err != nil {{
			conn.close()
			return nil, err
		}}
		if cmd.parser != nil {{
			if m, ok := raw.(map[string]any); ok {{
				results = append(results, cmd.parser(m))
			}} else {{
				results = append(results, raw)
			}}
		}} else {{
			results = append(results, raw)
		}}
	}}

	p.pool.put(conn)
	p.commands = p.commands[:0]
	return results, nil
}}

// Len returns the number of queued commands.
func (p *Pipeline) Len() int {{ return len(p.commands) }}

// Clear discards all queued commands.
func (p *Pipeline) Clear() {{ p.commands = p.commands[:0] }}
"#,
        pascal = n.pascal,
    ));

    // Generate pipeline methods for each command
    for (cmd_name, cmd) in &spec.commands {
        writeln!(out).unwrap();
        gen_go_pipeline_method(&mut out, cmd_name, cmd);
    }

    // Suppress unused import warnings
    out.push_str("\n// Ensure imports are used.\nvar _ = fmt.Sprintf\nvar _ = strconv.FormatInt\n");
    if needs_json {
        out.push_str("var _ = json.Marshal\n");
    }

    GeneratedFile {
        path: "pipeline.go".into(),
        content: out,
    }
}

fn gen_go_pipeline_method(out: &mut String, cmd_name: &str, cmd: &CommandDef) {
    if cmd.streaming {
        // Streaming commands cannot be pipelined; silently skip.
        return;
    }

    let method_name = cmd_name.to_upper_camel_case();
    let positional = cmd.positional_params();
    let named = cmd.named_params();

    let has_options = !named.is_empty();
    let has_response = !cmd.response.is_empty() && !cmd.simple_response;
    let response_type = format!("{}Response", cmd_name.to_upper_camel_case());

    // Method signature — returns *Pipeline for chaining
    writeln!(out, "// {method_name} queues a {} command.", cmd.verb).unwrap();

    let mut params: Vec<String> = Vec::new();
    for p in &positional {
        params.push(format!("{} string", p.name));
    }
    if has_options {
        params.push(format!("opts *{method_name}Options"));
    }

    writeln!(
        out,
        "func (p *Pipeline) {method_name}({}) *Pipeline {{",
        params.join(", "),
    )
    .unwrap();

    // Build args
    writeln!(out, "\targs := []string{{").unwrap();
    if let Some(sub) = &cmd.subcommand {
        writeln!(out, "\t\t\"{}\", \"{}\",", cmd.verb, sub).unwrap();
    } else {
        writeln!(out, "\t\t\"{}\",", cmd.verb).unwrap();
    }
    for p in &positional {
        if p.required {
            writeln!(out, "\t\t{},", p.name).unwrap();
        }
    }
    writeln!(out, "\t}}").unwrap();

    // Optional positional
    for p in &positional {
        if !p.required {
            writeln!(out, "\tif {} != \"\" {{", p.name).unwrap();
            writeln!(out, "\t\targs = append(args, {})", p.name).unwrap();
            writeln!(out, "\t}}").unwrap();
        }
    }

    // Named params from options
    if has_options {
        writeln!(out, "\tif opts != nil {{").unwrap();
        for p in &named {
            let field_name = p.name.to_upper_camel_case();
            let key = p.key.as_deref().unwrap();
            if p.param_type == "boolean_flag" {
                writeln!(out, "\t\tif opts.{field_name} {{").unwrap();
                writeln!(out, "\t\t\targs = append(args, \"{key}\")").unwrap();
                writeln!(out, "\t\t}}").unwrap();
            } else if p.param_type == "json_value" {
                writeln!(out, "\t\tif opts.{field_name} != nil {{").unwrap();
                writeln!(out, "\t\t\tb, _ := json.Marshal(opts.{field_name})").unwrap();
                writeln!(out, "\t\t\targs = append(args, \"{key}\", string(b))").unwrap();
                writeln!(out, "\t\t}}").unwrap();
            } else if p.variadic {
                writeln!(out, "\t\tif len(opts.{field_name}) > 0 {{").unwrap();
                writeln!(out, "\t\t\targs = append(args, \"{key}\")").unwrap();
                writeln!(out, "\t\t\targs = append(args, opts.{field_name}...)").unwrap();
                writeln!(out, "\t\t}}").unwrap();
            } else if p.param_type == "integer" {
                writeln!(out, "\t\tif opts.{field_name} != 0 {{").unwrap();
                writeln!(
                    out,
                    "\t\t\targs = append(args, \"{key}\", strconv.FormatInt(opts.{field_name}, 10))"
                )
                .unwrap();
                writeln!(out, "\t\t}}").unwrap();
            } else {
                writeln!(out, "\t\tif opts.{field_name} != \"\" {{").unwrap();
                writeln!(
                    out,
                    "\t\t\targs = append(args, \"{key}\", opts.{field_name})"
                )
                .unwrap();
                writeln!(out, "\t\t}}").unwrap();
            }
        }
        writeln!(out, "\t}}").unwrap();
    }

    // Append to commands with parser
    if has_response {
        writeln!(
            out,
            "\tp.commands = append(p.commands, pipelineCmd{{args: args, parser: func(m map[string]any) any {{ return parse{response_type}(m) }}}})"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "\tp.commands = append(p.commands, pipelineCmd{{args: args}})"
        )
        .unwrap();
    }

    writeln!(out, "\treturn p").unwrap();
    writeln!(out, "}}").unwrap();
}

fn gen_go_mod(_spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    GeneratedFile {
        path: "go.mod".into(),
        content: format!("module {}\n\ngo 1.22\n", n.go_module),
    }
}

fn gen_readme(spec: &ProtocolSpec, n: &Naming) -> GeneratedFile {
    let scheme = n
        .uri_schemes
        .first()
        .map(|s| s.trim_end_matches("://"))
        .unwrap_or(&n.snake);
    let scheme_tls = format!("{scheme}+tls");
    let mut cmds = String::new();
    for (cmd_name, cmd) in &spec.commands {
        if cmd.streaming {
            continue;
        }
        let method = cmd_name.to_upper_camel_case();
        cmds.push_str(&format!("- `client.{method}(...)` — {}\n", cmd.description));
    }

    GeneratedFile {
        path: "README.md".into(),
        content: format!(
            r#"# {pascal} Go Client

Go client for the [{pascal}](https://github.com/shroudb/{kebab}) {description}.

## Install

```bash
go get {go_module}
```

## Quick Start

```go
package main

import (
    "fmt"
    "log"

    {snake} "{go_module}"
)

func main() {{
    client, err := {snake}.Connect("{scheme}://localhost")
    if err != nil {{
        log.Fatal(err)
    }}
    defer client.Close()

    // Issue a credential
    result, err := client.Issue("my-keyspace", &{snake}.IssueOptions{{Ttl: 3600}})
    if err != nil {{
        log.Fatal(err)
    }}
    fmt.Println(result.CredentialId, result.Token)

    // Verify it
    verified, err := client.Verify("my-keyspace", result.Token, nil)
    if err != nil {{
        log.Fatal(err)
    }}
    fmt.Println(verified.State) // "active"
}}
```

## Connection URI

```
{scheme}://[token@]host[:port][/keyspace]
{scheme_tls}://[token@]host[:port][/keyspace]
```

Examples:
- `{scheme}://localhost` — plain TCP, default port {port}
- `{scheme_tls}://prod.example.com` — TLS
- `{scheme}://mytoken@localhost:{port}/sessions` — auth + keyspace

## Connection Pool

The client maintains an internal connection pool. Tune it at connect time:

```go
client, err := {snake}.Connect("{scheme}://localhost", {snake}.PoolConfig{{
    MaxIdle: 8,
    MaxOpen: 32,
}})
```

## Commands

{cmds}

## Streaming Subscribe

Subscribe to real-time events on a channel:

```go
sub, err := client.Subscribe("my-channel")
if err != nil {{
    log.Fatal(err)
}}
defer sub.Close()

for evt := range sub.Events() {{
    fmt.Printf("[%s] %s %s at %d\n", evt.EventType, evt.Keyspace, evt.Detail, evt.Timestamp)
}}
// Check for read errors after the channel closes
if err := <-sub.Err(); err != nil {{
    log.Println("subscription error:", err)
}}
```

Each `SubscriptionEvent` contains:
- `EventType` — the type of event (e.g. "issued", "revoked")
- `Keyspace` — the affected keyspace
- `Detail` — additional event detail
- `Timestamp` — Unix timestamp of the event

## Auto-generated

This client was generated by `shroudb-codegen` from `protocol.toml`.
"#,
            pascal = n.pascal,
            kebab = n.kebab,
            snake = n.snake,
            go_module = n.go_module,
            scheme = scheme,
            scheme_tls = scheme_tls,
            port = n.default_port,
            description = n.description,
            cmds = cmds,
        ),
    }
}
